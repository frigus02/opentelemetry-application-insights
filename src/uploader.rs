use crate::contracts::Envelope;
use reqwest::{blocking::Client, StatusCode};
use serde::Deserialize;

const URL: &str = "https://dc.services.visualstudio.com/v2/track";

#[derive(Debug, PartialEq)]
pub enum Response {
    Success,
    Retry,
    NoRetry,
}

/// Sends telemetry items to the server.
#[derive(Debug)]
pub(crate) struct Uploader {
    client: Client,
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

impl Uploader {
    /// Creates a new instance of telemetry items sender.
    pub(crate) fn new() -> Self {
        let client = Client::new();
        Self { client }
    }

    /// Sends a telemetry items to the server.
    pub(crate) fn send(&self, items: Vec<Envelope>) -> Response {
        let response = match self.client.post(URL).json(&items).send() {
            Ok(response) => response,
            Err(_err) => return Response::NoRetry,
        };
        match response.status() {
            StatusCode::OK => Response::Success,
            StatusCode::PARTIAL_CONTENT => {
                let content: Transmission = match response.json() {
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
            StatusCode::TOO_MANY_REQUESTS | StatusCode::REQUEST_TIMEOUT => Response::Retry,
            StatusCode::SERVICE_UNAVAILABLE => Response::Retry,
            StatusCode::INTERNAL_SERVER_ERROR => {
                if let Ok(content) = response.json::<Transmission>() {
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
}

/// Determines that a telemetry item can be re-send corresponding to this submission status
/// descriptor.
fn can_retry_item(item: &TransmissionItem) -> bool {
    item.status_code == StatusCode::PARTIAL_CONTENT
        || item.status_code == StatusCode::REQUEST_TIMEOUT
        || item.status_code == StatusCode::INTERNAL_SERVER_ERROR
        || item.status_code == StatusCode::SERVICE_UNAVAILABLE
        || item.status_code == StatusCode::TOO_MANY_REQUESTS
}
