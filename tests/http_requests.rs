//! Snapshot tests for generated HTTP requests
//!
//! # Update snapshots
//!
//! ```
//! INSTA_UPDATE=always cargo test
//! ```

use format::requests_to_string;
#[cfg(feature = "live-metrics")]
use opentelemetry::trace::{Span, Status};
use opentelemetry::{
    logs::{LogRecord as _, Logger as _, LoggerProvider as _, Severity},
    trace::{
        get_active_span, mark_span_as_active, Link, SpanKind, TraceContextExt, Tracer,
        TracerProvider,
    },
    Context, KeyValue,
};
use opentelemetry_application_insights::{attrs as ai, new_pipeline_from_connection_string};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions as semcov;
use recording_client::record;
use std::{collections::HashMap, time::Duration};
use tick::{AsyncStdTick, NoTick, TokioTick};

// Fake instrumentation key (this is a random uuid)
const CONNECTION_STRING: &str = "InstrumentationKey=0fdcec70-0ce5-4085-89d9-9ae8ead9af66";

#[test]
fn traces() {
    let requests = record(NoTick, |client| {
        // Fake instrumentation key (this is a random uuid)
        let client_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client.clone())
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default().with_resource(
                    Resource::builder_empty()
                        .with_attributes(vec![
                            KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                            KeyValue::new(semcov::resource::SERVICE_NAME, "client"),
                            KeyValue::new(semcov::resource::DEVICE_ID, "123"),
                            KeyValue::new(semcov::resource::DEVICE_MODEL_NAME, "device"),
                        ])
                        .build(),
                ),
            )
            .build_simple();
        let client_tracer = client_provider.tracer("test");

        let server_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default().with_resource(
                    Resource::builder_empty()
                        .with_attributes(vec![
                            KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                            KeyValue::new(semcov::resource::SERVICE_NAME, "server"),
                        ])
                        .build(),
                ),
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
                KeyValue::new(semcov::trace::NETWORK_PEER_ADDRESS, "10.1.2.4"),
                KeyValue::new(semcov::trace::HTTP_RESPONSE_STATUS_CODE, 200),
                KeyValue::new(semcov::attribute::USER_ID, "marry"),
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
                    KeyValue::new(semcov::trace::NETWORK_PEER_ADDRESS, "10.1.2.2"),
                    KeyValue::new(semcov::trace::USER_AGENT_ORIGINAL, "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0"),
                    KeyValue::new(semcov::attribute::USER_ID,"marry"),
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
                    let async_op_builder = server_tracer
                        .span_builder("async operation")
                        .with_links(vec![Link::new(span.span_context().clone(), Vec::new(), 0)]);
                    let async_op_context = Context::new();
                    let _span =
                        server_tracer.build_with_context(async_op_builder, &async_op_context);
                });
            }

            // Force the server span to be sent before the client span. Without this on Jan's PC
            // the server span gets sent after the client span, but on GitHub Actions it's the
            // other way around.
            std::thread::sleep(Duration::from_secs(1));
        }

        client_provider.shutdown().unwrap();
        server_provider.shutdown().unwrap();
    });
    let traces = requests_to_string(requests);
    insta::assert_snapshot!(traces);
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

        tracer_provider.shutdown().unwrap();
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

        tracer_provider.shutdown().unwrap();
    });
    let traces_batch_tokio = requests_to_string(requests);
    insta::assert_snapshot!(traces_batch_tokio);
}

#[test]
fn traces_with_resource_attributes_in_events() {
    let requests = record(NoTick, |client| {
        let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client)
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default().with_resource(
                    Resource::builder_empty()
                        .with_attribute(KeyValue::new("attr", "value"))
                        .build(),
                ),
            )
            .with_resource_attributes_in_events(true)
            .build_simple();
        let tracer = tracer_provider.tracer("test");

        tracer.in_span("resource attributes in events", |_cx| {
            get_active_span(|span| {
                span.add_event("An event!", vec![]);
            });
        });

        tracer_provider.shutdown().unwrap();
    });
    let traces_with_resource_attributes_in_events = requests_to_string(requests);
    insta::assert_snapshot!(traces_with_resource_attributes_in_events);
}

#[test]
fn logs() {
    let requests = record(NoTick, |client| {
        // Setup tracing
        let tracer_provider = new_pipeline_from_connection_string(CONNECTION_STRING)
            .expect("connection string is valid")
            .with_client(client.clone())
            .build_simple();
        let tracer = tracer_provider.tracer("test");

        // Setup logging
        let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
            CONNECTION_STRING,
            client,
        )
        .expect("connection string is valid");
        let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                Resource::builder_empty()
                    .with_attributes(vec![
                        KeyValue::new(semcov::resource::SERVICE_NAMESPACE, "test"),
                        KeyValue::new(semcov::resource::SERVICE_NAME, "client"),
                    ])
                    .build(),
            )
            .build();

        let otel_log_appender =
            opentelemetry_appender_log::OpenTelemetryLogBridge::new(&logger_provider);
        log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
        log::set_max_level(log::Level::Info.to_level_filter());

        let fruit = "apple";
        let price = 2.99;
        let colors = ("red", "green");
        let stock = HashMap::from([("red", 4)]);
        log::info!(fruit, price, colors:sval, stock:sval; "info! {fruit} is {price}");
        log::warn!("warn!");
        log::error!("error!");

        let logger = logger_provider.logger("test");
        let mut record = logger.create_log_record();
        record.set_severity_number(Severity::Fatal);
        record.add_attribute(semcov::trace::EXCEPTION_TYPE, "Foo");
        record.add_attribute(semcov::trace::EXCEPTION_MESSAGE, "Foo broke");
        record.add_attribute(semcov::trace::EXCEPTION_STACKTRACE, "A stack trace");
        logger.emit(record);

        tracer.in_span("span_with_logs", |_cx| {
            log::info!("with span");
        });

        logger_provider.shutdown().unwrap();
        tracer_provider.shutdown().unwrap();
    });
    let logs = requests_to_string(requests);
    insta::assert_snapshot!(logs);
}

#[test]
fn logs_with_resource_attributes_in_events() {
    let requests = record(NoTick, |client| {
        let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
            CONNECTION_STRING,
            client,
        )
        .expect("connection string is valid")
        .with_resource_attributes_in_events(true);
        let logger_provider = opentelemetry_sdk::logs::SdkLoggerProvider::builder()
            .with_batch_exporter(exporter)
            .with_resource(
                Resource::builder_empty()
                    .with_attribute(KeyValue::new("attr", "value"))
                    .build(),
            )
            .build();

        let logger = logger_provider.logger("test");
        let mut record = logger.create_log_record();
        record.set_body("message".into());
        logger.emit(record);

        logger_provider.shutdown().unwrap();
    });
    let logs_with_resource_attributes_in_events = requests_to_string(requests);
    insta::assert_snapshot!(logs_with_resource_attributes_in_events);
}

#[tokio::test]
#[cfg(feature = "live-metrics")]
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

        tracer_provider.shutdown().unwrap();
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
        requests: Arc<Mutex<Vec<Request<Bytes>>>>,
        tick: Arc<dyn Tick>,
    }

    #[async_trait]
    impl HttpClient for RecordingClient {
        async fn send_bytes(&self, req: Request<Bytes>) -> Result<Response<Bytes>, HttpError> {
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
    ) -> Vec<Request<Bytes>> {
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
    use bytes::Bytes;
    use flate2::read::GzDecoder;
    use http::{HeaderName, Request};
    use opentelemetry::Key;
    use opentelemetry_sdk::resource::{ResourceDetector, TelemetryResourceDetector};
    use opentelemetry_semantic_conventions as semcov;
    use regex::Regex;
    use std::sync::OnceLock;

    pub fn requests_to_string(requests: Vec<Request<Bytes>>) -> String {
        requests
            .into_iter()
            .map(request_to_string)
            .collect::<Vec<_>>()
            .join("\n\n\n")
    }

    fn request_to_string(req: Request<Bytes>) -> String {
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
        struct Strip {
            re: Regex,
            replacement: &'static str,
        }
        impl Strip {
            fn new(re: &str) -> Self {
                Self {
                    re: Regex::new(re).unwrap(),
                    replacement: r#"$prefix"$field": "STRIPPED""#,
                }
            }

            fn json_in_json(mut self) -> Self {
                self.replacement = r#"$prefix\"$field\":\"STRIPPED\""#;
                self
            }

            fn strip(&self, s: &str) -> String {
                self.re.replace_all(s, self.replacement).into()
            }
        }
        let otel_version = TelemetryResourceDetector
            .detect()
            .get(&Key::from_static_str(
                semcov::resource::TELEMETRY_SDK_VERSION,
            ))
            .expect("TelemetryResourceDetector provides TELEMETRY_SDK_VERSION")
            .to_string();
        static STRIP_CONFIGS: OnceLock<Vec<Strip>> = OnceLock::new();
        let configs = STRIP_CONFIGS.get_or_init(|| {
            vec![
                Strip::new(r#""(?P<field>time)": "\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}.\d{3}Z""#),
                Strip::new(r#""(?P<field>duration)": "\d+\.\d{2}:\d{2}:\d{2}\.\d{6}""#),
                Strip::new(r#""(?P<field>id|ai\.operation\.parentId)": "[a-f0-9]{16}""#),
                Strip::new(r#""(?P<field>ai\.operation\.id)": "[a-f0-9]{32}""#),
                Strip::new(r#""(?P<field>StreamId)": "[a-f0-9]{32}""#),
                Strip::new(r#"\\"(?P<field>operation_Id)\\":\\"[a-f0-9]{32}\\""#).json_in_json(),
                Strip::new(r#"\\"(?P<field>id)\\":\\"[a-f0-9]{16}\\""#).json_in_json(),
                Strip::new(r#""(?P<field>Timestamp)": "/Date\(\d+\)/""#),
                Strip::new(r#"(?P<prefix>"\\\\Processor\(_Total\)\\\\% Processor Time",\s*)"(?P<field>Value)": \d+\.\d+"#),
                Strip::new(r#"(?P<prefix>"\\\\Memory\\\\Committed Bytes",\s*)"(?P<field>Value)": \d+\.\d+"#),
                Strip::new(&format!(r#""(?P<field>telemetry\.sdk\.version)": "{otel_version}""#)),
                Strip::new(&format!(r#""(?P<field>ai\.internal\.sdkVersion)": "opentelemetry:{otel_version}""#)),
                Strip::new(&format!(r#""(?P<field>Version)": "opentelemetry:{otel_version}""#)),
            ]
        });

        configs
            .iter()
            .fold(body.into(), |body, config| config.strip(&body))
    }

    fn pretty_print_json(body: &[u8]) -> String {
        let gzip_decoder = GzDecoder::new(body);
        let json: serde_json::Value =
            serde_json::from_reader(gzip_decoder).expect("body is valid json");
        serde_json::to_string_pretty(&json).unwrap()
    }
}
