use crate::{models::Envelope, Error, StreamingHttpClient};
use bytes::Bytes;
#[cfg(feature = "metrics")]
use flate2::{write::GzEncoder, Compression};
use futures_util::io::Cursor;
use http::{Request, Response, Uri};
use serde::Deserialize;
use std::convert::TryFrom;
#[cfg(feature = "metrics")]
use std::io::{Read, Write};

const STATUS_OK: u16 = 200;
const STATUS_PARTIAL_CONTENT: u16 = 206;
const STATUS_REQUEST_TIMEOUT: u16 = 408;
const STATUS_TOO_MANY_REQUESTS: u16 = 429;
const STATUS_APPLICATION_INACTIVE: u16 = 439; // Quota
const STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
const STATUS_SERVICE_UNAVAILABLE: u16 = 503;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transmission {
    items_received: usize,
    items_accepted: usize,
    errors: Vec<TransmissionItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransmissionItem {
    status_code: u16,
}

/// Sends a telemetry items to the server.
pub(crate) async fn send<C: StreamingHttpClient>(
    client: &C,
    endpoint: &Uri,
    items: Vec<Envelope>,
) -> Result<(), Error> {
    let (first, rest) = match items.split_first() {
        Some((first, rest)) => (first, rest),
        None => return Ok(()),
    };
    let mut serialized = serde_json::to_vec(&first).map_err(Error::UploadSerializeRequest)?;
    for other in rest {
        serialized.push(u8::try_from('\n').unwrap());
        serialized.append(&mut serde_json::to_vec(&other).map_err(Error::UploadSerializeRequest)?);
    }
    let gzip_encoder =
        async_compression::futures::bufread::GzipEncoder::new(Cursor::new(serialized));

    let request = Request::post(endpoint)
        .header(http::header::CONTENT_TYPE, "application/x-json-stream")
        .header(http::header::CONTENT_ENCODING, "gzip")
        .body(gzip_encoder)
        .expect("request should be valid");

    // TODO Implement retries
    let response = client
        .send_streaming_2(request)
        .await
        .map_err(Error::UploadConnection)?;
    handle_response(response)
}

/// Sends a telemetry items to the server.
#[cfg(feature = "metrics")]
pub(crate) fn send_sync(endpoint: &Uri, items: Vec<Envelope>) -> Result<(), Error> {
    let payload = serialize_request_body(items)?;

    // TODO Implement retries
    let response = match ureq::post(&endpoint.to_string())
        .set(http::header::CONTENT_TYPE.as_str(), "application/json")
        .set(http::header::CONTENT_ENCODING.as_str(), "gzip")
        .send_bytes(&payload)
    {
        Ok(response) => response,
        Err(ureq::Error::Status(_, response)) => response,
        Err(ureq::Error::Transport(err)) => return Err(Error::UploadConnection(err.into())),
    };
    let status = response.status();
    let len = response
        .header(http::header::CONTENT_LENGTH.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();
    let mut bytes = Vec::with_capacity(len);
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| Error::UploadConnection(err.into()))?;
    handle_response(
        Response::builder()
            .status(status)
            .body(Bytes::from(bytes))
            .map_err(|err| Error::UploadConnection(err.into()))?,
    )
}

#[cfg(feature = "metrics")]
fn serialize_request_body(items: Vec<Envelope>) -> Result<Vec<u8>, Error> {
    // Weirdly gzip_encoder.write_all(serde_json::to_vec()) seems to be faster than
    // serde_json::to_writer(gzip_encoder). In a local test operating on items that result in
    // ~13MiB of JSON, this is what I've seen:
    // gzip_encoder.write_all(serde_json::to_vec()): 159ms
    // serde_json::to_writer(gzip_encoder):          247ms
    let serialized = serde_json::to_vec(&items).map_err(Error::UploadSerializeRequest)?;
    let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
    gzip_encoder
        .write_all(&serialized)
        .map_err(Error::UploadCompressRequest)?;
    gzip_encoder.finish().map_err(Error::UploadCompressRequest)
}

fn handle_response(response: Response<Bytes>) -> Result<(), Error> {
    match response.status().as_u16() {
        STATUS_OK => Ok(()),
        status @ STATUS_PARTIAL_CONTENT => {
            let content: Transmission = serde_json::from_slice(response.body())
                .map_err(Error::UploadDeserializeResponse)?;
            if content.items_received == content.items_accepted {
                Ok(())
            } else if content.errors.iter().any(can_retry_item) {
                Err(Error::Upload(format!(
                    "{}: Some items may be retried. However we don't currently support this.",
                    status
                )))
            } else {
                Err(Error::Upload(format!(
                    "{}: No retry possible. Response: {:?}",
                    status, content
                )))
            }
        }
        status @ STATUS_REQUEST_TIMEOUT
        | status @ STATUS_TOO_MANY_REQUESTS
        | status @ STATUS_APPLICATION_INACTIVE
        | status @ STATUS_SERVICE_UNAVAILABLE => {
            // TODO Implement retries
            Err(Error::Upload(format!("{}: Retry possible", status)))
        }
        status @ STATUS_INTERNAL_SERVER_ERROR => {
            if let Ok(content) = serde_json::from_slice::<Transmission>(response.body()) {
                if content.errors.iter().any(can_retry_item) {
                    Err(Error::Upload(format!(
                        "{}: Some items may be retried. However we don't currently support this.",
                        status
                    )))
                } else {
                    Err(Error::Upload(format!("{}: No retry possible", status)))
                }
            } else {
                // TODO Implement retries
                Err(Error::Upload(format!(
                    "{}: Some items may be retried",
                    status
                )))
            }
        }
        status => Err(Error::Upload(format!(
            "{}: No retry possible {}",
            status,
            String::from_utf8(response.body().to_vec()).unwrap()
        ))),
    }
}

/// Determines that a telemetry item can be re-send corresponding to this submission status
/// descriptor.
fn can_retry_item(item: &TransmissionItem) -> bool {
    item.status_code == STATUS_PARTIAL_CONTENT
        || item.status_code == STATUS_REQUEST_TIMEOUT
        || item.status_code == STATUS_TOO_MANY_REQUESTS
        || item.status_code == STATUS_APPLICATION_INACTIVE
        || item.status_code == STATUS_INTERNAL_SERVER_ERROR
        || item.status_code == STATUS_SERVICE_UNAVAILABLE
}
