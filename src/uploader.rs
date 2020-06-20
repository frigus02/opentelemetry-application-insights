use crate::models::Envelope;
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
        .timeout_connect(10_000)
        .timeout_read(10_000)
        .send_json(payload);
    match response.status() {
        STATUS_OK => Response::Success,
        STATUS_PARTIAL_CONTENT => {
            let content: Transmission = match response.into_json_deserialize() {
                Ok(content) => content,
                Err(_err) => return Response::NoRetry,
            };
            if content.items_received == content.items_accepted {
                Response::Success
            } else if content.errors.iter().any(|item| can_retry_item(item)) {
                Response::Retry
            } else {
                Response::NoRetry
            }
        }
        STATUS_TOO_MANY_REQUESTS | STATUS_REQUEST_TIMEOUT => Response::Retry,
        STATUS_SERVICE_UNAVAILABLE => Response::Retry,
        STATUS_INTERNAL_SERVER_ERROR => {
            if let Ok(content) = response.into_json_deserialize::<Transmission>() {
                if content.errors.iter().any(|item| can_retry_item(item)) {
                    Response::Retry
                } else {
                    Response::NoRetry
                }
            } else {
                Response::Retry
            }
        }
        _ => Response::NoRetry,
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
