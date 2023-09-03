#[cfg(feature = "live-metrics")]
use crate::models::QuickPulseEnvelope;
use crate::{models::Envelope, Error, HttpClient};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use http::{Request, Response, Uri};
use serde::Deserialize;
use std::io::Write;

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
pub(crate) async fn send(
    client: &dyn HttpClient,
    endpoint: &Uri,
    items: Vec<Envelope>,
) -> Result<(), Error> {
    let payload = serialize_envelopes(items)?;
    let request = Request::post(endpoint)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_ENCODING, "gzip")
        .body(payload)
        .expect("request should be valid");

    // TODO Implement retries
    let response = client
        .send(request)
        .await
        .map_err(Error::UploadConnection)?;
    handle_response(response)
}

fn serialize_envelopes(items: Vec<Envelope>) -> Result<Vec<u8>, Error> {
    // Weirdly gzip_encoder.write_all(serde_json::to_vec()) seems to be faster than
    // serde_json::to_writer(gzip_encoder). In a local test operating on items that result in
    // ~13MiB of JSON, this is what I've seen:
    // gzip_encoder.write_all(serde_json::to_vec()): 159ms
    // serde_json::to_writer(gzip_encoder):          247ms
    let serialized = serde_json::to_vec(&items).map_err(Error::UploadSerializeRequest)?;
    serialize_request_body(serialized)
}

fn serialize_request_body(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
    gzip_encoder
        .write_all(&data)
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
        status => Err(Error::Upload(format!("{}: No retry possible", status))),
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

#[cfg(feature = "live-metrics")]
pub(crate) enum PostOrPing {
    Post,
    Ping,
}

#[cfg(feature = "live-metrics")]
impl std::fmt::Display for PostOrPing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PostOrPing::Post => "post",
            PostOrPing::Ping => "ping",
        })
    }
}

#[cfg(feature = "live-metrics")]
pub(crate) struct QuickPulseResponse {
    pub(crate) should_post: bool,
    pub(crate) redirected_host: Option<http::Uri>,
    pub(crate) polling_interval_hint: Option<std::time::Duration>,
}

#[cfg(feature = "live-metrics")]
pub(crate) async fn send_quick_pulse(
    client: &dyn HttpClient,
    endpoint: &Uri,
    instrumentation_key: &str,
    post_or_ping: PostOrPing,
    envelope: QuickPulseEnvelope,
) -> Result<QuickPulseResponse, Error> {
    use std::convert::TryFrom;

    let endpoint = format!(
        "{}/QuickPulseService.svc/{}?ikey={}",
        endpoint, post_or_ping, instrumentation_key
    );
    let serialized = match post_or_ping {
        PostOrPing::Post => serde_json::to_vec(&[&envelope]),
        PostOrPing::Ping => serde_json::to_vec(&envelope),
    }
    .map_err(Error::UploadSerializeRequest)?;
    let payload = serialize_request_body(serialized)?;
    let mut request_builder = Request::post(endpoint)
        .header(http::header::EXPECT, "100-continue")
        .header(
            "x-ms-qps-transmission-time",
            quick_pulse_transmission_time(),
        )
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_ENCODING, "gzip");
    if matches!(post_or_ping, PostOrPing::Ping) {
        request_builder = request_builder
            .header("m-ms-qps-stream-id", envelope.stream_id)
            .header("m-ms-qps-machine-name", envelope.machine_name)
            .header("m-ms-qps-instance-name", envelope.instance)
            .header("m-ms-qps-invariant-version", envelope.invariant_version);
        if let Some(role_name) = envelope.role_name {
            request_builder = request_builder.header("m-ms-qps-role-name", role_name);
        }
    }
    let request = request_builder
        .body(payload)
        .expect("request should be valid");

    // TODO: post authorixation handler
    // https://github.com/microsoft/ApplicationInsights-node.js/blob/84d57aa1565ca8c3dff1e14aa8f63f00b8f87d34/Library/QuickPulseSender.ts#L93-L105

    // TODO Implement retries
    let response = client
        .send(request)
        .await
        .map_err(Error::UploadConnection)?;

    if response.status().as_u16() == STATUS_OK {
        let should_post = response
            .headers()
            .get("x-ms-qps-subscribed")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false);
        let redirected_host = response
            .headers()
            .get("x-ms-qps-service-endpoint-redirect-v2")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| http::Uri::try_from(v).ok());
        let polling_interval_hint = response
            .headers()
            .get("x-ms-qps-service-endpoint-interval-hint")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .map(std::time::Duration::from_millis);
        Ok(QuickPulseResponse {
            should_post,
            redirected_host,
            polling_interval_hint,
        })
    } else {
        Err(Error::Upload(String::new()))
    }
}

#[cfg(feature = "live-metrics")]
fn quick_pulse_transmission_time() -> String {
    use std::time::SystemTime;

    let ms_since_0001 = 62135596800000u128;
    let ms_since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    ((ms_since_0001 + ms_since_epoch) * 10_000).to_string()
}
