use crate::{models::Envelope, Error, HttpClient};
use backon::{ExponentialBuilder, FuturesTimerSleeper, RetryableWithContext};
use bytes::Bytes;
use flate2::{write::GzEncoder, Compression};
use http::{Request, Response, Uri};
use serde::{Deserialize, Serialize};
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

const RETRY_MIN_DELAY: Duration = Duration::from_millis(500);
const RETRY_MAX_DELAY: Duration = Duration::from_secs(5);
const RETRY_TOTAL_DELAY: Duration = Duration::from_secs(35);

/// Response containing the status of each telemetry item.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackResponse {
    /// The number of items received.
    items_received: usize,
    /// The number of items accepted.
    items_accepted: usize,
    /// An array of error detail objects.
    errors: Vec<ErrorDetails>,
}

/// The error details.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorDetails {
    /// The index in the original payload of the item.
    index: usize,
    /// The item specific HTTP Response status code.
    status_code: u16,
}

/// Sends a telemetry items to the server.
pub(crate) async fn send(
    client: &dyn HttpClient,
    endpoint: &Uri,
    items: Vec<Envelope>,
    retry_notify: Option<Arc<Mutex<dyn FnMut(&Error, Duration) + Send + 'static>>>,
) -> Result<(), Error> {
    let attempt = |mut items: Vec<Envelope>| async {
        match send_internal(client, endpoint, &items).await {
            result @ Ok(()) => (Vec::new(), result),
            result @ Err(UploadError::RetryAll(_)) => (items, result),
            Err(UploadError::RetrySome { err, to_retry, .. }) => {
                let mut index: usize = 0;
                items.retain(|_| {
                    let retry = to_retry.contains(&index);
                    index += 1;
                    retry
                });
                if items.is_empty() {
                    return (items, Ok(()));
                }
                (items, Err(UploadError::RetrySome { err, to_retry }))
            }
            result @ Err(_) => (Vec::new(), result),
        }
    };

    let (_, result) = attempt
        .retry(
            ExponentialBuilder::new()
                .with_min_delay(RETRY_MIN_DELAY)
                .with_max_delay(RETRY_MAX_DELAY)
                .with_jitter()
                // No max delay or max times should needed, because the batch span processor already
                // enforces a `max_export_timeout`. However, as of `opentelemetry_sdk` v0.30.0:
                // - the option is only respected for ::span_processor_with_async_runtime::BatchSpanProcessor
                // - the option doesn't exist for metric or log exports or the SimpleSpanProcessor
                // Therefore, add a total delay here, which is slightly larger than the default
                // `max_export_timeout`.
                .without_max_times()
                .with_total_delay(Some(RETRY_TOTAL_DELAY)),
        )
        .sleep(FuturesTimerSleeper)
        .context(items)
        .when(|err| {
            matches!(
                err,
                UploadError::RetryAll(_) | UploadError::RetrySome { .. }
            )
        })
        .notify(|error, duration| {
            if let Some(ref notify) = retry_notify {
                let mut notify = notify.lock().unwrap();
                notify(error.error(), duration);
            }
        })
        .await;
    result.map_err(|err| err.into_error())
}

async fn send_internal(
    client: &dyn HttpClient,
    endpoint: &Uri,
    items: &[Envelope],
) -> Result<(), UploadError> {
    let payload = Bytes::from(serialize_envelopes(items).map_err(|err| UploadError::Fatal(err))?);

    let request = Request::post(endpoint)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::CONTENT_ENCODING, "gzip")
        .body(payload)
        .expect("request should be valid");

    let response = client
        .send_bytes(request)
        .await
        .map_err(|err| UploadError::RetryAll(Error::UploadConnection(err)))?;

    handle_upload_response(response)
}

fn serialize_envelopes(items: &[Envelope]) -> Result<Vec<u8>, Error> {
    let serialized = serde_json::to_vec(items).map_err(Error::UploadSerializeRequest)?;
    serialize_request_body(&serialized)
}

pub(crate) fn serialize_request_body(data: &[u8]) -> Result<Vec<u8>, Error> {
    // Weirdly gzip_encoder.write_all(serde_json::to_vec()) seems to be faster than
    // serde_json::to_writer(gzip_encoder). In a local test operating on items that result in
    // ~13MiB of JSON, this is what I've seen:
    // gzip_encoder.write_all(serde_json::to_vec()): 159ms
    // serde_json::to_writer(gzip_encoder):          247ms
    let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
    gzip_encoder
        .write_all(&data)
        .map_err(Error::UploadCompressRequest)?;
    gzip_encoder.finish().map_err(Error::UploadCompressRequest)
}

enum UploadError {
    RetryAll(Error),
    RetrySome {
        err: Error,
        to_retry: HashSet<usize>,
    },
    Fatal(Error),
}

impl UploadError {
    fn error(&self) -> &Error {
        match self {
            Self::RetryAll(err) => err,
            Self::RetrySome { err, .. } => err,
            Self::Fatal(err) => err,
        }
    }

    fn into_error(self) -> Error {
        match self {
            Self::RetryAll(err) => err,
            Self::RetrySome { err, .. } => err,
            Self::Fatal(err) => err,
        }
    }
}

fn handle_upload_response(response: Response<Bytes>) -> Result<(), UploadError> {
    match response.status().as_u16() {
        STATUS_OK => Ok(()),
        status_code @ STATUS_PARTIAL_CONTENT => {
            let content: TrackResponse = match serde_json::from_slice(response.body()) {
                Ok(content) => content,
                Err(err) => return Err(UploadError::Fatal(Error::UploadDeserializeResponse(err))),
            };

            if content.items_received == content.items_accepted {
                return Ok(());
            }

            let to_retry = content
                .errors
                .iter()
                .filter(|error| can_retry_status_code(error.status_code))
                .map(|error| error.index)
                .collect::<HashSet<_>>();
            if to_retry.is_empty() {
                Err(UploadError::Fatal(Error::Upload(format!(
                    "{status_code}: Accepted {}/{} items; none were retryable.",
                    content.items_accepted, content.items_received
                ))))
            } else {
                Err(UploadError::RetrySome {
                    err: status_code_error(status_code),
                    to_retry,
                })
            }
        }
        status_code @ (STATUS_REQUEST_TIMEOUT
        | STATUS_TOO_MANY_REQUESTS
        | STATUS_APPLICATION_INACTIVE
        | STATUS_SERVICE_UNAVAILABLE) => Err(UploadError::RetryAll(status_code_error(status_code))),
        status_code @ STATUS_INTERNAL_SERVER_ERROR => {
            let content = match serde_json::from_slice::<TrackResponse>(response.body()) {
                Ok(content) => content,
                Err(_) => return Err(UploadError::RetryAll(status_code_error(status_code))),
            };

            let to_retry = content
                .errors
                .iter()
                .filter(|error| can_retry_status_code(error.status_code))
                .map(|error| error.index)
                .collect::<HashSet<_>>();
            if to_retry.is_empty() {
                Err(UploadError::Fatal(Error::Upload(format!(
                    "{status_code}: Accepted {}/{} items; none were retryable.",
                    content.items_accepted, content.items_received
                ))))
            } else {
                Err(UploadError::RetrySome {
                    err: status_code_error(status_code),
                    to_retry,
                })
            }
        }
        status_code => Err(UploadError::Fatal(status_code_error(status_code))),
    }
}

fn can_retry_status_code(code: u16) -> bool {
    code == STATUS_PARTIAL_CONTENT
        || code == STATUS_REQUEST_TIMEOUT
        || code == STATUS_TOO_MANY_REQUESTS
        || code == STATUS_APPLICATION_INACTIVE
        || code == STATUS_INTERNAL_SERVER_ERROR
        || code == STATUS_SERVICE_UNAVAILABLE
}

fn status_code_error(status_code: u16) -> Error {
    Error::Upload(format!("{status_code}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bytes::Bytes;
    use flate2::read::GzDecoder;
    use http::{Request, Response};
    use opentelemetry_http::{HttpClient, HttpError};
    use std::{collections::VecDeque, sync::Mutex};

    #[derive(Default, Debug)]
    struct TestClient {
        requests: Mutex<Vec<Request<Bytes>>>,
        responses: Mutex<VecDeque<Result<Response<Bytes>, HttpError>>>,
    }

    impl TestClient {
        fn with_response(self, response: Result<Response<Bytes>, HttpError>) -> Self {
            self.responses.lock().unwrap().push_back(response);
            self
        }

        fn with_200(self) -> Self {
            self.with_response(Ok(Response::builder()
                .status(200)
                .body(Bytes::from("{}"))
                .expect("")))
        }

        fn with_206(self, track_response: TrackResponse) -> Self {
            self.with_response(Ok(Response::builder()
                .status(206)
                .body(Bytes::from(serde_json::to_vec(&track_response).unwrap()))
                .expect("")))
        }

        fn with_400(self) -> Self {
            self.with_response(Ok(Response::builder()
                .status(400)
                .body(Bytes::from("{}"))
                .expect("")))
        }

        fn with_connection_error(self) -> Self {
            self.with_response(Err("connection error".into()))
        }
    }

    #[async_trait]
    impl HttpClient for TestClient {
        async fn send_bytes(&self, req: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
            self.requests.lock().unwrap().push(req);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("not enough responses are set up")
        }
    }

    fn endpoint() -> Uri {
        Uri::from_static("https://example.com/track")
    }

    fn envelopes(n: usize) -> Vec<Envelope> {
        let mut items = Vec::with_capacity(n);
        for index in 0..n {
            items.push(Envelope {
                name: "Test",
                time: index.to_string().into(),
                sample_rate: None,
                i_key: None,
                tags: None,
                data: None,
            });
        }
        items
    }

    fn envelopes_ids_from_request_body(body: &[u8]) -> Vec<usize> {
        let gzip_decoder = GzDecoder::new(body);
        let mut envelopes: Vec<serde_json::Value> =
            serde_json::from_reader(gzip_decoder).expect("body is json array");
        envelopes
            .drain(..)
            .map(|envelope| {
                envelope
                    .as_object()
                    .unwrap()
                    .get("time")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            })
            .collect()
    }

    #[tokio::test]
    async fn success() {
        let client = TestClient::default().with_200();
        let result = send(&client, &endpoint(), envelopes(1), None).await;
        assert!(result.is_ok());
        assert_eq!(client.requests.lock().unwrap().len(), 1, "request count");
    }

    #[tokio::test]
    async fn success_partial_with_all_items() {
        let client = TestClient::default().with_206(TrackResponse {
            items_received: 2,
            items_accepted: 2,
            errors: Vec::new(),
        });
        let result = send(&client, &endpoint(), envelopes(2), None).await;
        assert!(result.is_ok());
        assert_eq!(client.requests.lock().unwrap().len(), 1, "request count");
    }

    #[tokio::test]
    async fn fatal() {
        let client = TestClient::default().with_400();
        let result = send(&client, &endpoint(), envelopes(1), None).await;
        assert!(result.is_err());
        assert_eq!(client.requests.lock().unwrap().len(), 1, "request count");
        assert_eq!(result.unwrap_err().to_string(), "upload failed with 400");
    }

    #[tokio::test]
    async fn retry_connection_error() {
        let client = TestClient::default().with_connection_error().with_200();
        let result = send(&client, &endpoint(), envelopes(1), None).await;
        assert!(result.is_ok());
        assert_eq!(client.requests.lock().unwrap().len(), 2, "request count");
    }

    #[tokio::test]
    async fn retry_partial_content() {
        let client = TestClient::default()
            .with_206(TrackResponse {
                items_received: 10,
                items_accepted: 6,
                errors: vec![
                    ErrorDetails {
                        index: 1,
                        status_code: 400,
                    },
                    ErrorDetails {
                        index: 7,
                        status_code: STATUS_REQUEST_TIMEOUT,
                    },
                    ErrorDetails {
                        index: 8,
                        status_code: STATUS_REQUEST_TIMEOUT,
                    },
                    ErrorDetails {
                        index: 9,
                        status_code: STATUS_REQUEST_TIMEOUT,
                    },
                ],
            })
            .with_206(TrackResponse {
                items_received: 3,
                items_accepted: 2,
                errors: vec![ErrorDetails {
                    index: 2,
                    status_code: STATUS_TOO_MANY_REQUESTS,
                }],
            })
            .with_200();
        let result = send(&client, &endpoint(), envelopes(10), None).await;
        assert!(result.is_ok());
        let requests = client.requests.lock().unwrap();
        assert_eq!(requests.len(), 3, "request count");
        let items0 = envelopes_ids_from_request_body(requests[0].body());
        assert_eq!(items0, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        let items1 = envelopes_ids_from_request_body(requests[1].body());
        assert_eq!(items1, vec![7, 8, 9]);
        let items2 = envelopes_ids_from_request_body(requests[2].body());
        assert_eq!(items2, vec![9]);
    }
}
