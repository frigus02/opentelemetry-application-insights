use crate::{models::Envelope, Error, StreamingBody, StreamingHttpClient};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use futures_util::Stream;
use http::{Request, Response, Uri};
use serde::Deserialize;
use std::convert::TryFrom;
#[cfg(feature = "metrics")]
use std::io::Read;
use std::io::Write;
use std::pin::Pin;

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

fn serialize_item_newline(item: Envelope, newline: bool) -> Result<Vec<u8>, std::io::Error> {
    let mut result = serde_json::to_vec(&item)?;
    if newline {
        result.insert(0, u8::try_from('\n').unwrap());
    }

    // TODO: should probably create GzEncoder only once and pipe the entire stream through it
    // rather than creating a new one for each item.
    let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
    gzip_encoder.write_all(&result)?;
    gzip_encoder.finish()
}

struct Body {
    items: Vec<Envelope>,
    started: bool,
}

impl Stream for Body {
    type Item = Result<Vec<u8>, std::io::Error>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = Pin::into_inner(self);
        let next = match this.items.pop() {
            Some(next) => serialize_item_newline(next, this.started),
            None => return std::task::Poll::Ready(None),
        };
        this.started = true;
        std::task::Poll::Ready(Some(next))
    }
}

/// Sends a telemetry items to the server.
pub(crate) async fn send<C: StreamingHttpClient>(
    client: &C,
    endpoint: &Uri,
    items: Vec<Envelope>,
) -> Result<(), Error> {
    let body: StreamingBody = Box::pin(Body {
        items,
        started: false,
    });

    let request = Request::post(endpoint)
        .header(http::header::CONTENT_TYPE, "application/x-json-stream")
        .header(http::header::CONTENT_ENCODING, "gzip")
        .body(body)
        .expect("request should be valid");

    // TODO Implement retries
    let response = client
        .send_streaming(request)
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
