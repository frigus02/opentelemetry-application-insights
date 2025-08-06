use crate::{models::Envelope, Error, HttpClient};
use backon::{ExponentialBuilder, RetryableWithContext};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use http::{Request, Response, Uri};
use serde::Deserialize;
use std::{
    collections::HashSet,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};

// We need these constants because HTTP 439 is not part of the official HTTP
// status code registry.
const STATUS_OK: u16 = 200;
const STATUS_PARTIAL_CONTENT: u16 = 206;
const STATUS_REQUEST_TIMEOUT: u16 = 408;
const STATUS_TOO_MANY_REQUESTS: u16 = 429;
const STATUS_APPLICATION_INACTIVE: u16 = 439; // Quota
const STATUS_INTERNAL_SERVER_ERROR: u16 = 500;
const STATUS_SERVICE_UNAVAILABLE: u16 = 503;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackResponse {
    errors: Vec<TelemetryErrorDetails>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryErrorDetails {
    /// Position of the item inside the batch that the service is replying to
    index: usize,
    /// Status code of the item that the service is replying to.
    status_code: u16,
}

async fn send_internal(
    client: &dyn HttpClient,
    endpoint: &Uri,
    items: &[Envelope],
) -> Result<RetryPlan, Error> {
    let payload = Bytes::from(serialize_envelopes(items)?);

    let request = Request::post(endpoint)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_ENCODING, "gzip")
        .body(payload)
        .expect("request should be valid");

    let response = client
        .send_bytes(request)
        .await
        .map_err(Error::UploadConnection)?;

    handle_upload_response(response)
}

/// Sends a telemetry items to the server.
pub(crate) async fn send(
    client: &dyn HttpClient,
    endpoint: &Uri,
    items: Vec<Envelope>,
    retry_notify: Option<Arc<Mutex<Box<dyn FnMut(&Error, Duration) + Send + 'static>>>>,
) -> Result<(), Error> {
    let attempt = |mut items: Vec<Envelope>| async {
        match send_internal(client, endpoint, &items).await {
            Ok(RetryPlan::Done) => (Vec::new(), Ok(())),

            Ok(RetryPlan::Retry {
                status_code: status,
                to_retry,
            }) => {
                items = items
                    .drain(..)
                    .enumerate()
                    .filter_map(|(index, envelope)| to_retry.contains(&index).then_some(envelope))
                    .collect();

                if items.is_empty() {
                    return (items, Ok(()));
                }

                (
                    items,
                    Err(Error::Upload {
                        status_code: status,
                        can_retry: true,
                    }),
                )
            }

            Err(err) => (items, Err(err)),
        }
    };

    let (_, result) = attempt
        .retry(
            ExponentialBuilder::new()
                .with_min_delay(RETRY_MIN_DELAY)
                .with_max_delay(RETRY_MAX_DELAY)
                .with_jitter()
                .without_max_times(),
        )
        .context(items)
        .when(can_retry_operation)
        .notify(|error, duration| {
            if let Some(ref notify) = retry_notify {
                let mut notify = notify.lock().unwrap();
                notify(error, duration);
            }
        })
        .await;
    result
}

const RETRY_MIN_DELAY: Duration = Duration::from_millis(500);
const RETRY_MAX_DELAY: Duration = Duration::from_secs(5);

fn serialize_envelopes(items: &[Envelope]) -> Result<Vec<u8>, Error> {
    // Weirdly gzip_encoder.write_all(serde_json::to_vec()) seems to be faster than
    // serde_json::to_writer(gzip_encoder). In a local test operating on items that result in
    // ~13MiB of JSON, this is what I've seen:
    // gzip_encoder.write_all(serde_json::to_vec()): 159ms
    // serde_json::to_writer(gzip_encoder):          247ms
    let serialized = serde_json::to_vec(items).map_err(Error::UploadSerializeRequest)?;
    serialize_request_body(&serialized)
}

pub(crate) fn serialize_request_body(data: &[u8]) -> Result<Vec<u8>, Error> {
    let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
    gzip_encoder
        .write_all(&data)
        .map_err(Error::UploadCompressRequest)?;
    gzip_encoder.finish().map_err(Error::UploadCompressRequest)
}

enum RetryPlan {
    Done,
    Retry {
        status_code: u16,
        to_retry: HashSet<usize>,
    },
}

fn handle_upload_response(response: Response<Bytes>) -> Result<RetryPlan, Error> {
    match response.status().as_u16() {
        STATUS_OK => Ok(RetryPlan::Done),
        STATUS_PARTIAL_CONTENT => {
            let content: TrackResponse = serde_json::from_slice(response.body())
                .map_err(Error::UploadDeserializeResponse)?;

            // Collect the indices of items whose status code is retryable.
            let to_retry = content
                .errors
                .iter()
                .filter(|error| can_retry_status_code(error.status_code))
                .map(|error| error.index)
                .collect::<HashSet<_>>();

            if to_retry.is_empty() {
                Ok(RetryPlan::Done)
            } else {
                Ok(RetryPlan::Retry {
                    status_code: response.status().as_u16(),
                    to_retry,
                })
            }
        }
        status_code @ (STATUS_REQUEST_TIMEOUT
        | STATUS_TOO_MANY_REQUESTS
        | STATUS_APPLICATION_INACTIVE
        | STATUS_SERVICE_UNAVAILABLE) => Err(Error::Upload {
            status_code,
            can_retry: true,
        }),
        status @ STATUS_INTERNAL_SERVER_ERROR => {
            // If parsing fails, still retry 500s.
            let can_retry = match serde_json::from_slice::<TrackResponse>(response.body()) {
                Ok(content) => content
                    .errors
                    .iter()
                    .any(|error| can_retry_status_code(error.status_code)),
                Err(_) => true,
            };
            Err(Error::Upload {
                status_code: status,
                can_retry,
            })
        }
        status => Err(Error::Upload {
            status_code: status,
            can_retry: false,
        }),
    }
}

/// Determines that a telemetry item can be re-send corresponding to this submission status
/// code.
fn can_retry_status_code(code: u16) -> bool {
    code == STATUS_PARTIAL_CONTENT
        || code == STATUS_REQUEST_TIMEOUT
        || code == STATUS_TOO_MANY_REQUESTS
        || code == STATUS_APPLICATION_INACTIVE
        || code == STATUS_INTERNAL_SERVER_ERROR
        || code == STATUS_SERVICE_UNAVAILABLE
}

fn can_retry_operation(error: &Error) -> bool {
    matches!(
        error,
        Error::UploadConnection(_)
            | Error::Upload {
                can_retry: true,
                ..
            }
    )
}
