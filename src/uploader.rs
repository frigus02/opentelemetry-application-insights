use crate::models::Envelope;
use crate::HttpClient;
use http::Request;
use opentelemetry::exporter::trace::ExportResult;
use serde::Deserialize;

const URL: &str = "https://dc.services.visualstudio.com/v2/track";
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
    index: usize,
    status_code: u16,
    message: String,
}

/// Sends a telemetry items to the server.
pub(crate) async fn send(client: &dyn HttpClient, items: Vec<Envelope>) -> ExportResult {
    let payload = serde_json::to_vec(&items)?;
    let request = Request::post(URL)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(payload)?;

    // TODO Implement retries
    let response = client.send(request).await?;
    match response.status().as_u16() {
        STATUS_OK => Ok(()),
        status @ STATUS_PARTIAL_CONTENT => {
            let content: Transmission = serde_json::from_slice(response.body())?;
            if content.items_received == content.items_accepted {
                Ok(())
            } else if content.errors.iter().any(|item| can_retry_item(item)) {
                Err(format!("Upload error {}. Some items may be retried. However we don't currently support this.", status).into())
            } else {
                Err(format!(
                    "Upload error {}. No retry possible. Response: {:?}",
                    status, content
                )
                .into())
            }
        }
        status @ STATUS_REQUEST_TIMEOUT
        | status @ STATUS_TOO_MANY_REQUESTS
        | status @ STATUS_APPLICATION_INACTIVE
        | status @ STATUS_SERVICE_UNAVAILABLE => {
            // TODO Implement retries
            Err(format!("Upload error {}. Retry possible", status).into())
        }
        status @ STATUS_INTERNAL_SERVER_ERROR => {
            if let Ok(content) = serde_json::from_slice::<Transmission>(response.body()) {
                if content.errors.iter().any(|item| can_retry_item(item)) {
                    Err(format!("Upload error {}. Some items may be retried. However we don't currently support this.", status).into())
                } else {
                    Err(format!("Upload error {}. No retry possible", status).into())
                }
            } else {
                // TODO Implement retries
                Err(format!("Upload error {}. Some items may be retried", status).into())
            }
        }
        status => Err(format!("Upload error {}. No retry possible", status).into()),
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
