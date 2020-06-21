use crate::models::Envelope;
use log::debug;
use serde::Deserialize;

const URL: &str = "https://dc.services.visualstudio.com/v2/track";
const STATUS_OK: u16 = 200;
const STATUS_PARTIAL_CONTENT: u16 = 206;
const STATUS_REQUEST_TIMEOUT: u16 = 408;
const STATUS_TOO_MANY_REQUESTS: u16 = 429;
const STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
const STATUS_SERVICE_UNAVAILABLE: u16 = 503;

#[derive(Debug, PartialEq)]
pub(crate) enum Response {
    Success,
    Retry,
    NoRetry,
}

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
    index: usize,
    status_code: u16,
    message: String,
}

/// Sends a telemetry items to the server.
pub(crate) fn send(items: Vec<Envelope>) -> Response {
    let payload = match serde_json::to_value(items) {
        Ok(payload) => payload,
        Err(_) => return Response::NoRetry,
    };
    let response = ureq::post(URL)
        .timeout_connect(5_000)
        .timeout_read(10_000)
        .timeout_write(10_000)
        .send_json(payload);
    match response.status() {
        STATUS_OK => {
            debug!("Upload successful");
            Response::Success
        }
        status @ STATUS_PARTIAL_CONTENT => {
            let content: Transmission = match response.into_json_deserialize() {
                Ok(content) => content,
                Err(_err) => return Response::NoRetry,
            };
            if content.items_received == content.items_accepted {
                debug!("Upload successful");
                Response::Success
            } else if content.errors.iter().any(|item| can_retry_item(item)) {
                debug!("Upload error {}. Some items may be retried", status);
                Response::Retry
            } else {
                debug!("Upload error {}. No retry possible", status);
                Response::NoRetry
            }
        }
        status @ STATUS_TOO_MANY_REQUESTS
        | status @ STATUS_REQUEST_TIMEOUT
        | status @ STATUS_SERVICE_UNAVAILABLE => {
            debug!("Upload error {}. Retry possible", status);
            Response::Retry
        }
        status @ STATUS_INTERNAL_SERVER_ERROR => {
            if let Ok(content) = response.into_json_deserialize::<Transmission>() {
                if content.errors.iter().any(|item| can_retry_item(item)) {
                    debug!("Upload error {}. Some items may be retried", status);
                    Response::Retry
                } else {
                    debug!("Upload error {}. No retry possible", status);
                    Response::NoRetry
                }
            } else {
                debug!("Upload error {}. Some items may be retried", status);
                Response::Retry
            }
        }
        status => {
            debug!("Upload error {}. No retry possible", status);
            Response::NoRetry
        }
    }
}

/// Determines that a telemetry item can be re-send corresponding to this submission status
/// descriptor.
fn can_retry_item(item: &TransmissionItem) -> bool {
    item.status_code == STATUS_PARTIAL_CONTENT
        || item.status_code == STATUS_REQUEST_TIMEOUT
        || item.status_code == STATUS_INTERNAL_SERVER_ERROR
        || item.status_code == STATUS_SERVICE_UNAVAILABLE
        || item.status_code == STATUS_TOO_MANY_REQUESTS
}
