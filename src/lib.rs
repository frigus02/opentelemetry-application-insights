//! An [Azure Application Insights] exporter implementation for [OpenTelemetry Rust].
//!
//! [Azure Application Insights]: https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview
//! [OpenTelemetry Rust]: https://github.com/open-telemetry/opentelemetry-rust
//!
//! **Disclaimer**: This is not an official Microsoft product.
//!
//! # Usage
//!
//! ## Trace
//!
//! This requires the **trace** feature (enabled by default).
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
//! ## Logs
//!
//! This requires the **logs** feature (enabled by default).
//!
//! ```no_run
//! use log::{Level, info};
//! use opentelemetry_appender_log::OpenTelemetryLogBridge;
//! use opentelemetry_sdk::logs::{LoggerProvider, BatchLogProcessor};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Setup exporter
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
//!         connection_string,
//!         reqwest::Client::new(),
//!     )
//!     .expect("valid connection string");
//!     let logger_provider = LoggerProvider::builder()
//!         .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
//!         .build();
//!     let otel_log_appender = OpenTelemetryLogBridge::new(&logger_provider);
//!     log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
//!     log::set_max_level(Level::Info.to_level_filter());
//!
//!     // Log
//!     let fruit = "apple";
//!     let price = 2.99;
//!     info!("{fruit} costs {price}");
//!
//!     // Export remaining logs before exiting
//!     let _ = logger_provider.shutdown();
//! }
//! ```
//!
//! ## Metrics
//!
//! This requires the **metrics** feature (enabled by default).
//!
//! ```no_run
//! use opentelemetry::global;
//! use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Setup exporter
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
//!         connection_string,
//!         reqwest::Client::new(),
//!     )
//!     .expect("valid connection string");
//!     let reader = PeriodicReader::builder(exporter, opentelemetry_sdk::runtime::Tokio).build();
//!     let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();
//!     global::set_meter_provider(meter_provider);
//!
//!     // Record value
//!     let meter = global::meter("example");
//!     let histogram = meter.f64_histogram("pi").init();
//!     histogram.record(3.14, &[]);
//!
//!     // Simulate work, during which metrics will periodically be reported.
//!     tokio::time::sleep(Duration::from_secs(300)).await;
//! }
//! ```
//!
#![cfg_attr(
    feature = "live-metrics",
    doc = r#"
## Live Metrics

This requires the **live-metrics** feature _and_ the `build_batch`/`install_batch` methods.

Enable live metrics collection: <https://learn.microsoft.com/en-us/azure/azure-monitor/app/live-stream>.

Metrics are based on traces. See attribute mapping below for how traces are mapped to requests,
dependencies and exceptions and how they are deemed "successful" or not.

To configure role, instance, and machine name provide `service.name`, `service.instance.id`, and
`host.name` resource attributes respectively in the trace config.

Sample telemetry is not supported, yet.

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
//! - [`reqwest`]: enable the **opentelemetry-http/reqwest** feature and configure the exporter
//!   with either `with_client(reqwest::Client::new())` or
//!   `with_client(reqwest::blocking::Client::new())`.
//! - and more...
//!
//! [`async-std`]: https://crates.io/crates/async-std
//! [`opentelemetry-http`]: https://crates.io/crates/opentelemetry-http
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`tokio`]: https://crates.io/crates/tokio
//!
//! Alternatively you can bring any other HTTP client by implementing the `HttpClient` trait.
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
//! | `user.id`                                                                  | Context: Authenticated user id (`ai.user.authUserId`)    |
//! | `SpanKind::Server` + `http.request.method` + `http.route`                  | Context: Operation Name (`ai.operation.name`)            |
//! | `ai.*`                                                                     | Context: AppInsights Tag (`ai.*`)                        |
//! | `url.full`                                                                 | Dependency Data                                          |
//! | `db.query.text`                                                            | Dependency Data                                          |
//! | `http.request.header.host`                                                 | Dependency Target                                        |
//! | `server.address` + `server.port`                                           | Dependency Target                                        |
//! | `network.peer.address` + `network.peer.port`                               | Dependency Target                                        |
//! | `db.namespace`                                                             | Dependency Target                                        |
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
//! | `user.id`                   | `enduser.id`                               |
//! | `db.namespace`              | `db.name`                                  |
//! | `db.query.text`             | `db.statement`                             |
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
#![doc(html_root_url = "https://docs.rs/opentelemetry-application-insights/0.35.0")]
#![allow(clippy::needless_doctest_main)]
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, deny(warnings))]

mod connection_string;
mod convert;
#[cfg(feature = "logs")]
mod logs;
#[cfg(feature = "metrics")]
mod metrics;
mod models;
#[cfg(feature = "live-metrics")]
mod quick_pulse;
#[cfg(doctest)]
mod readme_test;
mod tags;
#[cfg(feature = "trace")]
mod trace;
mod uploader;
#[cfg(feature = "live-metrics")]
mod uploader_quick_pulse;

#[cfg(feature = "live-metrics")]
use connection_string::DEFAULT_LIVE_ENDPOINT;
use connection_string::{ConnectionString, DEFAULT_BREEZE_ENDPOINT};
pub use models::context_tag_keys::attrs;
#[cfg(feature = "trace")]
use opentelemetry::{global, trace::TracerProvider as _, KeyValue, Value};
pub use opentelemetry_http::HttpClient;
use opentelemetry_sdk::export::ExportError;
#[cfg(any(feature = "trace", feature = "logs"))]
use opentelemetry_sdk::Resource;
#[cfg(feature = "trace")]
use opentelemetry_sdk::{
    runtime::RuntimeChannel,
    trace::{Config, Tracer, TracerProvider},
};
#[cfg(feature = "trace")]
use opentelemetry_semantic_conventions as semcov;
#[cfg(feature = "live-metrics")]
use quick_pulse::QuickPulseManager;
#[cfg(feature = "trace")]
use std::convert::TryInto;
use std::{error::Error as StdError, fmt::Debug, sync::Arc};

/// Create a new Application Insights exporter pipeline builder
#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
#[deprecated(
    since = "0.27.0",
    note = "use new_pipeline_from_connection_string() or new_pipeline_from_env() instead"
)]
pub fn new_pipeline(instrumentation_key: String) -> PipelineBuilder<()> {
    PipelineBuilder {
        client: (),
        config: None,
        endpoint: http::Uri::from_static(DEFAULT_BREEZE_ENDPOINT),
        #[cfg(feature = "live-metrics")]
        live_metrics_endpoint: http::Uri::from_static(DEFAULT_LIVE_ENDPOINT),
        #[cfg(feature = "live-metrics")]
        live_metrics: false,
        instrumentation_key,
        sample_rate: None,
    }
}

/// Create a new Application Insights exporter pipeline builder
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// let tracer = opentelemetry_application_insights::new_pipeline_from_connection_string(
///         "InstrumentationKey=...;IngestionEndpoint=https://westus2-0.in.applicationinsights.azure.com"
///     )?
///     .with_client(reqwest::blocking::Client::new())
///     .install_simple();
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
pub fn new_pipeline_from_connection_string(
    connection_string: impl AsRef<str>,
) -> Result<PipelineBuilder<()>, Box<dyn StdError + Send + Sync + 'static>> {
    let connection_string: ConnectionString = connection_string.as_ref().parse()?;
    Ok(PipelineBuilder {
        client: (),
        config: None,
        endpoint: connection_string.ingestion_endpoint,
        #[cfg(feature = "live-metrics")]
        live_metrics_endpoint: connection_string.live_endpoint,
        #[cfg(feature = "live-metrics")]
        live_metrics: false,
        instrumentation_key: connection_string.instrumentation_key,
        sample_rate: None,
    })
}

/// Create a new Application Insights exporter pipeline builder
///
/// Reads connection string from `APPLICATIONINSIGHTS_CONNECTION_STRING` environment variable.
///
/// ```no_run
/// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
/// let tracer = opentelemetry_application_insights::new_pipeline_from_env()?
///     .with_client(reqwest::blocking::Client::new())
///     .install_simple();
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
pub fn new_pipeline_from_env(
) -> Result<PipelineBuilder<()>, Box<dyn StdError + Send + Sync + 'static>> {
    let connection_string: ConnectionString =
        std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")?.parse()?;
    Ok(PipelineBuilder {
        client: (),
        config: None,
        endpoint: connection_string.ingestion_endpoint,
        #[cfg(feature = "live-metrics")]
        live_metrics_endpoint: connection_string.live_endpoint,
        #[cfg(feature = "live-metrics")]
        live_metrics: false,
        instrumentation_key: connection_string.instrumentation_key,
        sample_rate: None,
    })
}

/// Application Insights exporter pipeline builder
#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
#[derive(Debug)]
pub struct PipelineBuilder<C> {
    client: C,
    config: Option<Config>,
    endpoint: http::Uri,
    #[cfg(feature = "live-metrics")]
    live_metrics_endpoint: http::Uri,
    #[cfg(feature = "live-metrics")]
    live_metrics: bool,
    instrumentation_key: String,
    sample_rate: Option<f64>,
}

#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
impl<C> PipelineBuilder<C> {
    /// Set HTTP client, which the exporter will use to send telemetry to Application Insights.
    ///
    /// Use this to set an HTTP client which fits your async runtime.
    pub fn with_client<NC>(self, client: NC) -> PipelineBuilder<NC> {
        PipelineBuilder {
            client,
            config: self.config,
            endpoint: self.endpoint,
            #[cfg(feature = "live-metrics")]
            live_metrics_endpoint: self.live_metrics_endpoint,
            #[cfg(feature = "live-metrics")]
            live_metrics: self.live_metrics,
            instrumentation_key: self.instrumentation_key,
            sample_rate: self.sample_rate,
        }
    }

    /// Set endpoint used to ingest telemetry. This should consist of scheme and authrity. The
    /// exporter will call `/v2/track` on the specified endpoint.
    ///
    /// Default: <https://dc.services.visualstudio.com>
    ///
    /// Note: This example requires [`reqwest`] and the **opentelemetry-http/reqwest** feature.
    ///
    /// [`reqwest`]: https://crates.io/crates/reqwest
    ///
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    /// let tracer = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_endpoint("https://westus2-0.in.applicationinsights.azure.com")?
    ///     .install_simple();
    /// # Ok(())
    /// # }
    /// ```
    #[deprecated(
        since = "0.27.0",
        note = "use new_pipeline_from_connection_string() or new_pipeline_from_env() instead"
    )]
    pub fn with_endpoint(
        mut self,
        endpoint: &str,
    ) -> Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        self.endpoint = endpoint.try_into()?;
        Ok(self)
    }

    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    ///
    /// Note: This example requires [`reqwest`] and the **opentelemetry-http/reqwest** feature.
    ///
    /// [`reqwest`]: https://crates.io/crates/reqwest
    ///
    /// ```no_run
    /// let tracer = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_sample_rate(0.3)
    ///     .install_simple();
    /// ```
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = Some(sample_rate * 100.0);
        self
    }

    /// Assign the SDK config for the exporter pipeline.
    ///
    /// If there is an existing `sdk::Config` in the `PipelineBuilder` the `sdk::Resource`s
    /// are merged and any other parameters are overwritten.
    ///
    /// Note: This example requires [`reqwest`] and the **opentelemetry-http/reqwest** feature.
    ///
    /// [`reqwest`]: https://crates.io/crates/reqwest
    ///
    /// ```no_run
    /// # use opentelemetry::KeyValue;
    /// # use opentelemetry_sdk::Resource;
    /// let tracer = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_trace_config(opentelemetry_sdk::trace::config().with_resource(
    ///         Resource::new(vec![
    ///             KeyValue::new("service.name", "my-application"),
    ///         ]),
    ///     ))
    ///     .install_simple();
    /// ```
    pub fn with_trace_config(self, mut config: Config) -> Self {
        if let Some(old_config) = self.config {
            let merged_resource = old_config.resource.merge(config.resource.clone());
            config = config.with_resource(merged_resource);
        }

        PipelineBuilder {
            config: Some(config),
            ..self
        }
    }

    /// Assign the service name under which to group traces by adding a service.name
    /// `sdk::Resource` or overriding a previous setting of it.
    ///
    /// If a `sdk::Config` does not exist on the `PipelineBuilder` one will be created.
    ///
    /// This will be translated, along with the service namespace, to the Cloud Role Name.
    ///
    /// ```
    /// let tracer = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_service_name("my-application")
    ///     .install_simple();
    /// ```
    pub fn with_service_name<T: Into<Value>>(self, name: T) -> Self {
        let new_resource = Resource::new(vec![KeyValue::new(semcov::resource::SERVICE_NAME, name)]);
        let config = if let Some(old_config) = self.config {
            let merged_resource = old_config.resource.merge(&new_resource);
            old_config.with_resource(merged_resource)
        } else {
            Config::default().with_resource(new_resource)
        };

        PipelineBuilder {
            config: Some(config),
            ..self
        }
    }

    /// Enable live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    pub fn with_live_metrics(self, enable_live_metrics: bool) -> Self {
        PipelineBuilder {
            live_metrics: enable_live_metrics,
            ..self
        }
    }
}

#[cfg(feature = "trace")]
#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
impl<C> PipelineBuilder<C>
where
    C: HttpClient + 'static,
{
    fn init_exporter(self) -> Exporter<C> {
        Exporter {
            client: Arc::new(self.client),
            endpoint: Arc::new(
                append_v2_track(self.endpoint).expect("appending /v2/track should always work"),
            ),
            instrumentation_key: self.instrumentation_key,
            sample_rate: self.sample_rate.unwrap_or(100.0),
            resource: Resource::empty(),
        }
    }

    /// Build a configured `TracerProvider` with a simple span processor.
    pub fn build_simple(mut self) -> TracerProvider {
        let config = self.config.take();
        let exporter = self.init_exporter();
        let mut builder = TracerProvider::builder().with_simple_exporter(exporter);
        if let Some(config) = config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Build a configured `TracerProvider` with a batch span processor using the specified
    /// runtime.
    pub fn build_batch<R: RuntimeChannel>(mut self, runtime: R) -> TracerProvider {
        let config = self.config.take();
        #[cfg(feature = "live-metrics")]
        let live_metrics = self.live_metrics;
        #[cfg(feature = "live-metrics")]
        let live_metrics_endpoint = self.live_metrics_endpoint.clone();
        let exporter = self.init_exporter();
        let mut builder = TracerProvider::builder();
        #[cfg(feature = "live-metrics")]
        if live_metrics {
            let mut resource = Resource::default();
            if let Some(ref config) = config {
                resource = resource.merge(config.resource.as_ref());
            };
            builder = builder.with_span_processor(QuickPulseManager::new(
                exporter.client.clone(),
                live_metrics_endpoint,
                exporter.instrumentation_key.clone(),
                resource,
                runtime.clone(),
            ));
        }
        builder = builder.with_batch_exporter(exporter, runtime);
        if let Some(config) = config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `TracerProvider`. See the `build_simple` function if you don't need
    /// that.
    pub fn install_simple(self) -> Tracer {
        let trace_provider = self.build_simple();
        let tracer = trace_provider
            .tracer_builder("opentelemetry-application-insights")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_schema_url(semcov::SCHEMA_URL)
            .build();
        let _previous_provider = global::set_tracer_provider(trace_provider);
        tracer
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `TracerProvider`. See the `build_simple` function if you don't need
    /// that.
    pub fn install_batch<R: RuntimeChannel>(self, runtime: R) -> Tracer {
        let trace_provider = self.build_batch(runtime);
        let tracer = trace_provider
            .tracer_builder("opentelemetry-application-insights")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_schema_url(semcov::SCHEMA_URL)
            .build();
        let _previous_provider = global::set_tracer_provider(trace_provider);
        tracer
    }
}

/// Application Insights span exporter
pub struct Exporter<C> {
    client: Arc<C>,
    endpoint: Arc<http::Uri>,
    instrumentation_key: String,
    #[cfg(feature = "trace")]
    sample_rate: f64,
    #[cfg(any(feature = "trace", feature = "logs"))]
    resource: Resource,
}

impl<C: Debug> Debug for Exporter<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Exporter");
        debug
            .field("client", &self.client)
            .field("endpoint", &self.endpoint)
            .field("instrumentation_key", &self.instrumentation_key);
        #[cfg(feature = "trace")]
        debug.field("sample_rate", &self.sample_rate);
        debug.finish()
    }
}

impl<C> Exporter<C> {
    /// Create a new exporter.
    #[deprecated(since = "0.27.0", note = "use new_from_connection_string() instead")]
    pub fn new(instrumentation_key: String, client: C) -> Self {
        Self {
            client: Arc::new(client),
            endpoint: Arc::new(
                append_v2_track(DEFAULT_BREEZE_ENDPOINT)
                    .expect("appending /v2/track should always work"),
            ),
            instrumentation_key,
            #[cfg(feature = "trace")]
            sample_rate: 100.0,
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource: Resource::empty(),
        }
    }

    /// Create a new exporter.
    pub fn new_from_connection_string(
        connection_string: impl AsRef<str>,
        client: C,
    ) -> Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        let connection_string: ConnectionString = connection_string.as_ref().parse()?;
        Ok(Self {
            client: Arc::new(client),
            endpoint: Arc::new(
                append_v2_track(connection_string.ingestion_endpoint)
                    .expect("appending /v2/track should always work"),
            ),
            instrumentation_key: connection_string.instrumentation_key,
            #[cfg(feature = "trace")]
            sample_rate: 100.0,
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource: Resource::empty(),
        })
    }

    /// Set endpoint used to ingest telemetry. This should consist of scheme and authrity. The
    /// exporter will call `/v2/track` on the specified endpoint.
    ///
    /// Default: <https://dc.services.visualstudio.com>
    #[deprecated(since = "0.27.0", note = "use new_from_connection_string() instead")]
    pub fn with_endpoint(
        mut self,
        endpoint: &str,
    ) -> Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        self.endpoint = Arc::new(append_v2_track(endpoint)?);
        Ok(self)
    }

    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    #[cfg(feature = "trace")]
    #[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = sample_rate * 100.0;
        self
    }
}

fn append_v2_track(uri: impl ToString) -> Result<http::Uri, http::uri::InvalidUri> {
    uploader::append_path(uri, "v2/track")
}

/// Errors that occurred during span export.
#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Application Insights telemetry data failed to serialize to JSON. Telemetry reporting failed
    /// because of this.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("serializing upload request failed with {0}")]
    UploadSerializeRequest(serde_json::Error),

    /// Application Insights telemetry data failed serialize or compress. Telemetry reporting failed
    /// because of this.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("compressing upload request failed with {0}")]
    UploadCompressRequest(std::io::Error),

    /// Application Insights telemetry response failed to deserialize from JSON.
    ///
    /// Telemetry reporting may have worked. But since we could not look into the response, we
    /// can't be sure.
    ///
    /// Note: This is an error in this crate. If you spot this, please open an issue.
    #[error("deserializing upload response failed with {0}")]
    UploadDeserializeResponse(serde_json::Error),

    /// Could not complete the HTTP request to Application Insights to send telemetry data.
    /// Telemetry reporting failed because of this.
    #[error("sending upload request failed with {0}")]
    UploadConnection(Box<dyn StdError + Send + Sync + 'static>),

    /// Application Insights returned at least one error for the reported telemetry data.
    #[error("upload failed with {0}")]
    Upload(String),

    /// Failed to process span for live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    #[error("process span for live metrics failed with {0}")]
    QuickPulseProcessSpan(opentelemetry_sdk::runtime::TrySendError),

    /// Failed to stop live metrics.
    #[cfg(feature = "live-metrics")]
    #[cfg_attr(docsrs, doc(cfg(feature = "live-metrics")))]
    #[error("stop live metrics failed with {0}")]
    QuickPulseShutdown(opentelemetry_sdk::runtime::TrySendError),
}

impl ExportError for Error {
    fn exporter_name(&self) -> &'static str {
        "application-insights"
    }
}
