//! Snapshot tests for generated HTTP requests
//!
//! # Update snapshots
//!
//! ```
//! INSTA_UPDATE=always cargo test
//! ```

use format::requests_to_string;
use opentelemetry::{
    trace::{
        get_active_span, mark_span_as_active, Span, SpanKind, Status, TraceContextExt, Tracer,
        TracerProvider,
    },
    Context, KeyValue,
};
use opentelemetry_application_insights::{attrs as ai, new_pipeline_from_connection_string};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions as semcov;
use recording_client::record;
use std::time::Duration;
use tick::{AsyncStdTick, NoTick, TokioTick};

// Fake instrumentation key (this is a random uuid)
const CONNECTION_STRING: &str = "InstrumentationKey=0fdcec70-0ce5-4085-89d9-9ae8ead9af66";

#[test]
fn traces_simple() {
    let requests = record(NoTick, |client| {
        // Fake instrumentation key (this is a random uuid)
        let client_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client.clone())
            .with_trace_config(
                opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                    KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                    KeyValue::new(semcov::resource::SERVICE_NAME, "client"),
                    KeyValue::new(semcov::resource::DEVICE_ID, "123"),
                    KeyValue::new(semcov::resource::DEVICE_MODEL_NAME, "device"),
                ])),
            )
            .build_simple();
        let client_tracer = client_provider.tracer("test");

        let server_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .with_trace_config(
                opentelemetry_sdk::trace::config().with_resource(Resource::new(vec![
                    KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                    KeyValue::new(semcov::resource::SERVICE_NAME, "server"),
                ])),
            )
            .build_simple();
        let server_tracer = server_provider.tracer("test");

        // An HTTP client make a request
        let span = client_tracer
            .span_builder("dependency")
            .with_kind(SpanKind::Client)
            .with_attributes(vec![
                KeyValue::new(semcov::trace::HTTP_REQUEST_METHOD, "GET"),
                KeyValue::new(semcov::trace::NETWORK_PROTOCOL_NAME, "http"),
                KeyValue::new(semcov::trace::NETWORK_PROTOCOL_VERSION, "1.1"),
                KeyValue::new(
                    semcov::trace::URL_FULL,
                    "https://example.com:8080/hello/world?name=marry",
                ),
                KeyValue::new(semcov::trace::SERVER_ADDRESS, "example.com"),
                KeyValue::new(semcov::trace::SERVER_PORT, 8080),
                KeyValue::new(semcov::trace::SERVER_SOCKET_ADDRESS, "10.1.2.4"),
                KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
                KeyValue::new(semcov::trace::ENDUSER_ID, "marry"),
            ])
            .start(&client_tracer);
        {
            let cx = Context::current_with_span(span);
            let _client_guard = cx.clone().attach();
            // The server receives the request
            let builder = server_tracer
                .span_builder("request")
                .with_kind(SpanKind::Server)
                .with_attributes(vec![
                    KeyValue::new(semcov::trace::HTTP_REQUEST_METHOD, "GET"),
                    KeyValue::new(semcov::trace::NETWORK_PROTOCOL_NAME, "http"),
                    KeyValue::new(semcov::trace::NETWORK_PROTOCOL_VERSION, "1.1"),
                    KeyValue::new(semcov::trace::URL_PATH, "/hello/world"),
                    KeyValue::new(semcov::trace::URL_QUERY, "name=marry"),
                    KeyValue::new(semcov::trace::SERVER_ADDRESS, "example.com"),
                    KeyValue::new(semcov::trace::SERVER_PORT, 8080),
                    KeyValue::new(semcov::trace::URL_SCHEME, "https"),
                    KeyValue::new(semcov::trace::HTTP_ROUTE, "/hello/world"),
                    KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
                    KeyValue::new(semcov::trace::CLIENT_ADDRESS, "10.1.2.3"),
                    KeyValue::new(semcov::trace::CLIENT_SOCKET_ADDRESS, "10.1.2.2"),
                    KeyValue::new(semcov::trace::USER_AGENT_ORIGINAL, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0"),
                    KeyValue::new(semcov::trace::ENDUSER_ID,"marry"),
                ]);
            let span = server_tracer.build_with_context(builder, &cx);
            {
                let _server_guard = mark_span_as_active(span);
                get_active_span(|span| {
                    span.add_event(
                        "An event!",
                        vec![
                            KeyValue::new("happened", true),
                            // Emulate tracing level
                            // https://docs.rs/tracing-core/0.1.30/src/tracing_core/metadata.rs.html#531
                            KeyValue::new("level", "WARN"),
                        ],
                    );
                    span.add_event(
                        "ai.custom",
                        vec![
                            KeyValue::new(ai::CUSTOM_EVENT_NAME, "A custom event!"),
                            KeyValue::new("happened", true),
                        ],
                    );
                    let error: Box<dyn std::error::Error> = "An error".into();
                    span.record_error(error.as_ref());
                });
            }

            // Force the server span to be sent before the client span. Without this on Jan's PC
            // the server span gets sent after the client span, but on GitHub Actions it's the
            // other way around.
            std::thread::sleep(Duration::from_secs(1));
        }
    });
    let traces_simple = requests_to_string(requests);
    insta::assert_snapshot!(traces_simple);
}

#[async_std::test]
async fn traces_batch_async_std() {
    let requests = record(AsyncStdTick, |client| {
        let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .build_batch(opentelemetry_sdk::runtime::AsyncStd);
        let tracer = tracer_provider.tracer("test");

        tracer.in_span("async-std", |_cx| {});
    });
    let traces_batch_async_std = requests_to_string(requests);
    insta::assert_snapshot!(traces_batch_async_std);
}

#[tokio::test]
async fn traces_batch_tokio() {
    let requests = record(TokioTick, |client| {
        let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .build_batch(opentelemetry_sdk::runtime::TokioCurrentThread);
        let tracer = tracer_provider.tracer("test");

        tracer.in_span("tokio", |_cx| {});
    });
    let traces_batch_tokio = requests_to_string(requests);
    insta::assert_snapshot!(traces_batch_tokio);
}

#[tokio::test]
async fn live_metrics() {
    let requests = record(TokioTick, |client| {
        let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .with_live_metrics(true)
            .build_batch(opentelemetry_sdk::runtime::TokioCurrentThread);
        let tracer = tracer_provider.tracer("test");

        // Wait for one ping request so we start to collect metrics.
        std::thread::sleep(Duration::from_secs(6));

        {
            let _span = tracer
                .span_builder("live-metrics")
                .with_kind(SpanKind::Server)
                .start(&tracer);
            let _span = tracer
                .span_builder("live-metrics")
                .with_kind(SpanKind::Server)
                .with_status(Status::error(""))
                .start(&tracer);
            let mut span = tracer
                .span_builder("live-metrics")
                .with_kind(SpanKind::Client)
                .with_status(Status::error(""))
                .start(&tracer);
            let error: Box<dyn std::error::Error> = "An error".into();
            span.record_error(error.as_ref());
        }

        // Wait for two pong requests.
        std::thread::sleep(Duration::from_secs(2));
    });
    let live_metrics = requests_to_string(requests);
    insta::assert_snapshot!(live_metrics);
}

mod recording_client {
    use super::tick::Tick;
    use async_trait::async_trait;
    use bytes::Bytes;
    use http::{Request, Response};
    use opentelemetry_http::{HttpClient, HttpError};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    #[derive(Debug, Clone)]
    pub struct RecordingClient {
        requests: Arc<Mutex<Vec<Request<Vec<u8>>>>>,
        tick: Arc<dyn Tick>,
    }

    #[async_trait]
    impl HttpClient for RecordingClient {
        async fn send(&self, req: Request<Vec<u8>>) -> Result<Response<Bytes>, HttpError> {
            self.tick.tick().await;

            let is_live_metrics = req.uri().path().contains("QuickPulseService.svc");
            let res = if is_live_metrics {
                Response::builder()
                    .status(200)
                    .header("x-ms-qps-subscribed", "true")
                    .header(
                        "x-ms-qps-service-endpoint-redirect-v2",
                        "https://redirected",
                    )
                    .header("x-ms-qps-service-endpoint-interval-hint", "500")
                    .body(Bytes::new())
                    .expect("response is fell formed")
            } else {
                Response::builder()
                    .status(200)
                    .body(Bytes::from("{}"))
                    .expect("response is fell formed")
            };

            self.requests
                .lock()
                .expect("requests mutex is healthy")
                .push(req);
            Ok(res)
        }
    }

    pub fn record(
        tick: impl Tick + 'static,
        generate_fn: impl Fn(RecordingClient),
    ) -> Vec<Request<Vec<u8>>> {
        let requests = Arc::new(Mutex::new(Vec::new()));
        generate_fn(RecordingClient {
            requests: Arc::clone(&requests),
            tick: Arc::new(tick),
        });

        // Give async runtime some time to quit. I don't see any way to properly wait for tasks
        // spawned with async-std.
        std::thread::sleep(Duration::from_secs(1));

        Arc::try_unwrap(requests)
            .expect("client is dropped everywhere")
            .into_inner()
            .expect("requests mutex is healthy")
    }
}

mod tick {
    use async_trait::async_trait;
    use std::{fmt::Debug, time::Duration};

    #[async_trait]
    pub trait Tick: Debug + Send + Sync {
        async fn tick(&self);
    }

    #[derive(Debug)]
    pub struct NoTick;

    #[async_trait]
    impl Tick for NoTick {
        async fn tick(&self) {}
    }

    #[derive(Debug)]
    pub struct AsyncStdTick;

    #[async_trait]
    impl Tick for AsyncStdTick {
        async fn tick(&self) {
            async_std::task::sleep(Duration::from_millis(1)).await;
        }
    }

    #[derive(Debug)]
    pub struct TokioTick;

    #[async_trait]
    impl Tick for TokioTick {
        async fn tick(&self) {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    }
}

mod format {
    use flate2::read::GzDecoder;
    use http::{HeaderName, Request};
    use regex::Regex;

    pub fn requests_to_string(requests: Vec<Request<Vec<u8>>>) -> String {
        requests
            .into_iter()
            .map(request_to_string)
            .collect::<Vec<_>>()
            .join("\n\n\n")
    }

    fn request_to_string(req: Request<Vec<u8>>) -> String {
        let method = req.method();
        let path = req.uri().path_and_query().expect("path exists");
        let version = format!("{:?}", req.version());
        let host = req.uri().authority().expect("authority exists");
        let headers = req
            .headers()
            .into_iter()
            .map(|(name, value)| {
                let value = value.to_str().expect("header value is valid string");
                format!("{}: {}", name, strip_changing_header(name, value))
            })
            .collect::<Vec<_>>()
            .join("\n");
        let body = strip_changing_values(&pretty_print_json(req.body()));
        format!("{method} {path} {version}\nhost: {host}\n{headers}\n\n{body}")
    }

    fn strip_changing_header<'a>(name: &HeaderName, value: &'a str) -> &'a str {
        if name == "x-ms-qps-transmission-time" || name == "x-ms-qps-stream-id" {
            "STRIPPED"
        } else {
            value
        }
    }

    fn strip_changing_values(body: &str) -> String {
        let res = vec![
            Regex::new(r#""(?P<field>time)": "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z""#)
                .unwrap(),
            Regex::new(r#""(?P<field>duration)": "\d+\.\d{2}:\d{2}:\d{2}\.\d{6}""#).unwrap(),
            Regex::new(r#""(?P<field>id|ai\.operation\.parentId)": "[a-z0-9]{16}""#).unwrap(),
            Regex::new(r#""(?P<field>ai\.operation\.id)": "[a-z0-9]{32}""#).unwrap(),
            Regex::new(r#""(?P<field>StreamId)": "[a-z0-9]{32}""#).unwrap(),
            Regex::new(r#""(?P<field>Timestamp)": "/Date\(\d+\)/""#).unwrap(),
            Regex::new(
                r#"(?P<prefix>"\\\\Processor\(_Total\)\\\\% Processor Time",\s*)"(?P<field>Value)": \d+\.\d+"#,
            ).unwrap(),
            Regex::new(
                r#"(?P<prefix>"\\\\Memory\\\\Committed Bytes",\s*)"(?P<field>Value)": \d+\.\d+"#,
            ).unwrap(),
        ];

        res.into_iter().fold(body.into(), |body, re| {
            re.replace_all(&body, r#"$prefix"$field": "STRIPPED""#)
                .into()
        })
    }

    fn pretty_print_json(body: &[u8]) -> String {
        let gzip_decoder = GzDecoder::new(body);
        let json: serde_json::Value =
            serde_json::from_reader(gzip_decoder).expect("body is valid json");
        serde_json::to_string_pretty(&json).unwrap()
    }
}
