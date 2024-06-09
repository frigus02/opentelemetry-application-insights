//! An [Azure Application Insights] exporter implementation for [OpenTelemetry Rust].
//!
//! [Azure Application Insights]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview
//! [OpenTelemetry Rust]: https://github.com/open-telemetry/opentelemetry-rust
//!
//! **Disclaimer**: This is not an official Microsoft product.
//!
//! # Usage
//!
//! Configure a OpenTelemetry pipeline using the Application Insights exporter and start creating
//! spans (this example requires the **opentelemetry-http/reqwest** feature):
//!
//! ```no_run
//! use opentelemetry::trace::Tracer as _;
//!
//! fn main() {
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let tracer = opentelemetry_application_insights::new_pipeline_from_connection_string(connection_string)
//!         .expect("valid connection string")
//!         .with_client(reqwest::blocking::Client::new())
//!         .install_simple();
//!
//!     tracer.in_span("main", |_cx| {});
//! }
//! ```
//!
//! ## Simple or Batch
//!
//! The functions `build_simple` and `install_simple` build/install a trace pipeline using the
//! simple span processor. This means each span is processed and exported synchronously at the time
//! it ends.
//!
//! The functions `build_batch` and `install_batch` use the batch span processor instead. This
//! means spans are exported periodically in batches, which can be better for performance. This
//! feature requires an async runtime such as Tokio or async-std. If you decide to use a batch span
//! processor, make sure to call `opentelemetry::global::shutdown_tracer_provider()` before your
//! program exits to ensure all remaining spans are exported properly (this example requires the
//! **opentelemetry/rt-tokio** and **opentelemetry-http/reqwest** features).
//!
//! ```no_run
//! use opentelemetry::trace::Tracer as _;
//!
//! #[tokio::main]
//! async fn main() {
//!     let tracer = opentelemetry_application_insights::new_pipeline_from_env()
//!         .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING is valid connection string")
//!         .with_client(reqwest::Client::new())
//!         .install_batch(opentelemetry_sdk::runtime::Tokio);
//!
//!     tracer.in_span("main", |_cx| {});
//!
//!     opentelemetry::global::shutdown_tracer_provider();
//! }
//! ```
//!
//! ## Async runtimes and HTTP clients
//!
//! In order to support different async runtimes, the exporter requires you to specify an HTTP
//! client that works with your chosen runtime. The [`opentelemetry-http`] crate comes with support
//! for:
//!
//! - [`isahc`]: enable the **opentelemetry-http/isahc** feature and configure the exporter with
//!   `with_client(isahc::HttpClient::new()?)`.
//! - [`reqwest`]: enable the **opentelemetry-http/reqwest** feature and configure the exporter
//!   with either `with_client(reqwest::Client::new())` or
//!   `with_client(reqwest::blocking::Client::new())`.
//! - and more...
//!
//! [`async-std`]: https://crates.io/crates/async-std
//! [`opentelemetry-http`]: https://crates.io/crates/opentelemetry-http
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`isahc`]: https://crates.io/crates/isahc
//! [`tokio`]: https://crates.io/crates/tokio
//!
//! Alternatively you can bring any other HTTP client by implementing the `HttpClient` trait.
//!
#![cfg_attr(
    feature = "logs",
    doc = r#"
## Logs

This requires the **logs** feature.

```no_run
use log::{Level, info};
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_sdk::logs::{LoggerProvider, BatchLogProcessor};

#[tokio::main]
async fn main() {
    // Setup exporter
    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::Client::new(),
    )
    .expect("valid connection string");
    let logger_provider = LoggerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();
    let otel_log_appender = OpenTelemetryLogBridge::new(&logger_provider);
    log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
    log::set_max_level(Level::Info.to_level_filter());

    // Log
    let fruit = "apple";
    let price = 2.99;
    info!("{fruit} costs {price}");

    // Export remaining logs before exiting
    let _ = logger_provider.shutdown();
}
```
"#
)]
#![cfg_attr(
    feature = "metrics",
    doc = r#"
## Metrics

This requires the **metrics** feature.

```no_run
use opentelemetry::global;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Setup exporter
    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::Client::new(),
    )
    .expect("valid connection string");
    let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();
    let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();
    global::set_meter_provider(meter_provider);

    // Record value
    let meter = global::meter("example");
    let histogram = meter.f64_histogram("pi").init();
    histogram.record(3.14, &[]);

    // Simulate work, during which metrics will periodically be reported.
    tokio::time::sleep(Duration::from_secs(300)).await;
}
```
"#
)]
#![cfg_attr(
    feature = "live-metrics",
    doc = r#"
## Live Metrics

Enable live metrics collection: <https://learn.microsoft.com/en-us/azure/azure-monitor/app/live-stream>.

Metrics are based on traces. See attribute mapping below for how traces are mapped to requests,
dependencies and exceptions and how they are deemed "successful" or not.

To configure role, instance, and machine name provide `service.name`, `service.instance.id`, and
`host.name` resource attributes respectively in the trace config.

Sample telemetry is not supported, yet.

This requires the **live-metrics** feature _and_ the `build_batch`/`install_batch` methods.

```no_run
use opentelemetry::trace::Tracer as _;

#[tokio::main]
async fn main() {
    let tracer = opentelemetry_application_insights::new_pipeline_from_env()
        .expect("env var APPLICATIONINSIGHTS_CONNECTION_STRING is valid connection string")
        .with_client(reqwest::Client::new())
        .with_live_metrics(true)
        .install_batch(opentelemetry_sdk::runtime::Tokio);

    // ... send traces ...

    opentelemetry::global::shutdown_tracer_provider();
}
```
"#
)]
//!
//! # Attribute mapping
//!
//! OpenTelemetry and Application Insights are using different terminology. This crate tries its
//! best to map OpenTelemetry fields to their correct Application Insights pendant.
//!
//! - [OpenTelemetry specification: Span](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#span)
//! - [Application Insights data model](https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model)
//!
//! ## Resource
//!
//! Resource and instrumentation library attributes map to the following fields for spans, events
//! and metrics (the mapping tries to follow the OpenTelemetry semantic conventions for [resource]).
//!
//! [resource]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/resource/semantic_conventions
//!
//! | OpenTelemetry attribute key                    | Application Insights field                               |
//! | ---------------------------------------------- | -------------------------------------------------------- |
//! | `service.namespace` + `service.name`           | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.deployment.name`                          | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.replicaset.name`                          | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.statefulset.name`                         | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.job.name`                                 | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.cronjob.name`                             | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.daemonset.name`                           | Context: Cloud role (`ai.cloud.role`)                    |
//! | `k8s.pod.name`                                 | Context: Cloud role instance (`ai.cloud.roleInstance`)   |
//! | `service.instance.id`                          | Context: Cloud role instance (`ai.cloud.roleInstance`)   |
//! | `device.id`                                    | Context: Device id (`ai.device.id`)                      |
//! | `device.model.name`                            | Context: Device model (`ai.device.model`)                |
//! | `service.version`                              | Context: Application version (`ai.application.ver`)      |
//! | `telemetry.sdk.name` + `telemetry.sdk.version` | Context: Internal SDK version (`ai.internal.sdkVersion`) |
//! | `ai.*`                                         | Context: AppInsights Tag (`ai.*`)                        |
//!
//! If `service.name` is the default (i.e. starts with "unknown_service:"), the Kubernetes based
//! values take precedence.
//!
//! ## Spans
//!
//! The OpenTelemetry SpanKind determines the Application Insights telemetry type:
//!
//! | OpenTelemetry SpanKind           | Application Insights telemetry type |
//! | -------------------------------- | ----------------------------------- |
//! | `CLIENT`, `PRODUCER`, `INTERNAL` | [Dependency]                        |
//! | `SERVER`, `CONSUMER`             | [Request]                           |
//!
//! The Span's status determines the Success field of a Dependency or Request. Success is `false` if
//! the status `Error`; otherwise `true`.
//!
//! The following of the Span's attributes map to special fields in Application Insights (the
//! mapping tries to follow the OpenTelemetry semantic conventions for [trace]).
//!
//! Note: for `INTERNAL` Spans the Dependency Type is always `"InProc"`.
//!
//! [trace]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions
//! [Dependency]: https://learn.microsoft.com/en-us/azure/azure-monitor/app/data-model-dependency-telemetry
//! [Request]: https://learn.microsoft.com/en-us/azure/azure-monitor/app/data-model-request-telemetry
//!
//! | OpenTelemetry attribute key                                                | Application Insights field                               |
//! | -------------------------------------------------------------------------- | -------------------------------------------------------- |
//! | `enduser.id`                                                               | Context: Authenticated user id (`ai.user.authUserId`)    |
//! | `SpanKind::Server` + `http.request.method` + `http.route`                  | Context: Operation Name (`ai.operation.name`)            |
//! | `ai.*`                                                                     | Context: AppInsights Tag (`ai.*`)                        |
//! | `url.full`                                                                 | Dependency Data                                          |
//! | `db.statement`                                                             | Dependency Data                                          |
//! | `http.request.header.host`                                                 | Dependency Target                                        |
//! | `server.address` + `server.port`                                           | Dependency Target                                        |
//! | `network.peer.address` + `network.peer.port`                               | Dependency Target                                        |
//! | `db.name`                                                                  | Dependency Target                                        |
//! | `http.response.status_code`                                                | Dependency Result code                                   |
//! | `db.system`                                                                | Dependency Type                                          |
//! | `messaging.system`                                                         | Dependency Type                                          |
//! | `rpc.system`                                                               | Dependency Type                                          |
//! | `"HTTP"` if any `http.` attribute exists                                   | Dependency Type                                          |
//! | `"DB"` if any `db.` attribute exists                                       | Dependency Type                                          |
//! | `url.full`                                                                 | Request Url                                              |
//! | `url.scheme` + `http.request.header.host` + `url.path` + `url.query`       | Request Url                                              |
//! | `url.scheme` + `server.address` + `server.port` + `url.path` + `url.query` | Request Url                                              |
//! | `client.address`                                                           | Request Source                                           |
//! | `network.peer.address`                                                     | Request Source                                           |
//! | `http.response.status_code`                                                | Request Response code                                    |
//!
//! All other attributes are directly converted to custom properties.
//!
//! For Requests the attributes `http.request.method` and `http.route` override the Name.
//!
//! ### Deprecated attributes
//!
//! The following deprecated attributes also work:
//!
//! | Attribute                   | Deprecated attribute                       |
//! | --------------------------- | ------------------------------------------ |
//! | `http.request.method`       | `http.method`                              |
//! | `http.request.header.host`  | `http.host`                                |
//! | `http.response.status_code` | `http.status_code`                         |
//! | `url.full`                  | `http.url`                                 |
//! | `url.scheme`                | `http.scheme`                              |
//! | `url.path` + `url.query`    | `http.target`                              |
//! | `client.address`            | `http.client_ip`                           |
//! | `network.peer.address`      | `server.socket.address` (for client spans) |
//! | `network.peer.address`      | `net.sock.peer.addr`    (for client spans) |
//! | `network.peer.address`      | `net.peer.ip`           (for client spans) |
//! | `network.peer.port`         | `server.socket.port`    (for client spans) |
//! | `network.peer.port`         | `net.sock.peer.port`    (for client spans) |
//! | `network.peer.address`      | `client.socket.address` (for server spans) |
//! | `network.peer.address`      | `net.sock.peer.addr`    (for server spans) |
//! | `network.peer.address`      | `net.peer.ip`           (for server spans) |
//! | `server.address`            | `net.peer.name`         (for client spans) |
//! | `server.port`               | `net.peer.port`         (for client spans) |
//! | `server.address`            | `net.host.name`         (for server spans) |
//! | `server.port`               | `net.host.port`         (for server spans) |
//!
//! ## Events
//!
//! Events are converted into [Exception] telemetry if the event name equals `"exception"` (see
//! OpenTelemetry semantic conventions for [exceptions]) with the following mapping:
//!
//! | OpenTelemetry attribute key | Application Insights field |
//! | --------------------------- | -------------------------- |
//! | `exception.type`            | Exception type             |
//! | `exception.message`         | Exception message          |
//! | `exception.stacktrace`      | Exception call stack       |
//!
//! Events are converted into [Event] telemetry if the event name equals `"ai.custom"` with the
//! following mapping:
//!
//! | OpenTelemetry attribute key | Application Insights field |
//! | --------------------------- | -------------------------- |
//! | `ai.customEvent.name`       | Event name                 |
//!
//! All other events are converted into [Trace] telemetry with the following mapping:
//!
//! | OpenTelemetry attribute key  | Application Insights field |
//! | ---------------------------- | -------------------------- |
//! | `level` ([`tracing::Level`]) | Severity level             |
//!
//! All other attributes are directly converted to custom properties.
//!
//! [exceptions]: https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/exceptions.md
//! [Exception]: https://learn.microsoft.com/en-us/azure/azure-monitor/app/data-model-exception-telemetry
//! [Event]: https://learn.microsoft.com/en-us/azure/azure-monitor/app/data-model-event-telemetry
//! [Trace]: https://learn.microsoft.com/en-us/azure/azure-monitor/app/data-model-trace-telemetry
//! [`tracing::Level`]: https://docs.rs/tracing/0.1.37/tracing/struct.Level.html
//!
//! ## Logs
//!
//! Logs are reported similar to events:
//!
//! - If they contain an `exception.type` or `exception.message` attribute, they're converted to
//!   [Exception] telemetry with the same attribute mapping as events.
//! - Otherwise they're converted to [Trace] telemetry.
//!
//! ## Metrics
//!
//! Metrics get reported to Application Insights as Metric Data. The [`Aggregation`] determines how
//! the data is represented.
//!
//! | Aggregator           | Data representation                                                  |
//! | -------------------- | -------------------------------------------------------------------- |
//! | Histogram            | aggregation with sum, count, min, and max (buckets are not exported) |
//! | ExponentialHistogram | aggregation with sum, count, min, and max (buckets are not exported) |
//! | Gauge                | one measurement                                                      |
//! | Sum                  | aggregation with only a value                                        |
//!
//! [`Aggregation`]: https://docs.rs/opentelemetry/0.20.0/opentelemetry/sdk/metrics/data/trait.Aggregation.html
#![doc(html_root_url = "https://docs.rs/opentelemetry-application-insights/0.33.0")]
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, deny(warnings))]

mod connection_string;
mod convert;
mod error;
mod exporter;
#[cfg(feature = "logs")]
mod logs;
#[cfg(feature = "metrics")]
mod metrics;
mod models;
mod pipeline;
#[cfg(feature = "live-metrics")]
mod quick_pulse;
#[cfg(doctest)]
mod readme_test;
mod tags;
mod trace;
mod uploader;
#[cfg(feature = "live-metrics")]
mod uploader_quick_pulse;

pub use error::Error;
pub use exporter::{new_exporter_from_connection_string, new_exporter_from_env, Exporter};
pub use models::context_tag_keys::attrs;
pub use opentelemetry_http::HttpClient;
pub use pipeline::{new_pipeline, LogsPipeline, MetricsPipeline, Pipeline, TracesPipeline};
