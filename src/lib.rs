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
//! This requires the **trace** (enabled by default) and **opentelemetry-http/reqwest** features.
//!
//! ```no_run
//! use opentelemetry::{global, trace::Tracer};
//! use opentelemetry_sdk::trace::SdkTracerProvider;
//!
//! fn main() {
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
//!         connection_string,
//!         reqwest::blocking::Client::new(),
//!     )
//!     .expect("valid connection string");
//!     let tracer_provider = SdkTracerProvider::builder()
//!         .with_batch_exporter(exporter)
//!         .build();
//!     global::set_tracer_provider(tracer_provider.clone());
//!
//!     let tracer = global::tracer("example");
//!     tracer.in_span("main", |_cx| {});
//!
//!     tracer_provider.shutdown().unwrap();
//! }
//! ```
//!
//! ## Logs
//!
//! This requires the **logs** (enabled by default) and **opentelemetry-http/reqwest** features.
//!
//! ```no_run
//! use log::{Level, info};
//! use opentelemetry_appender_log::OpenTelemetryLogBridge;
//! use opentelemetry_sdk::logs::SdkLoggerProvider;
//!
//! fn main() {
//!     // Setup exporter
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
//!         connection_string,
//!         reqwest::blocking::Client::new(),
//!     )
//!     .expect("valid connection string");
//!     let logger_provider = SdkLoggerProvider::builder()
//!         .with_batch_exporter(exporter)
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
//!     logger_provider.shutdown().unwrap();
//! }
//! ```
//!
//! ## Metrics
//!
//! This requires the **metrics** (enabled by default) and **opentelemetry-http/reqwest** features.
//!
//! ```no_run
//! use opentelemetry::global;
//! use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
//! use std::time::Duration;
//!
//! fn main() {
//!     // Setup exporter
//!     let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
//!     let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
//!         connection_string,
//!         reqwest::blocking::Client::new(),
//!     )
//!     .expect("valid connection string");
//!     let reader = PeriodicReader::builder(exporter).build();
//!     let meter_provider = SdkMeterProvider::builder().with_reader(reader).build();
//!     global::set_meter_provider(meter_provider.clone());
//!
//!     // Record value
//!     let meter = global::meter("example");
//!     let histogram = meter.f64_histogram("pi").build();
//!     histogram.record(3.14, &[]);
//!
//!     // Simulate work, during which metrics will periodically be reported.
//!     std::thread::sleep(Duration::from_secs(300));
//!
//!     meter_provider.shutdown().unwrap();
//! }
//! ```
//!
#![cfg_attr(
    feature = "live-metrics",
    doc = r#"
## Live Metrics

This requires the **live-metrics** feature and the experimental async runtime span processor API behind the **opentelemetry_sdk/experimental_trace_batch_span_processor_with_async_runtime** feature.

Enable live metrics collection: <https://learn.microsoft.com/en-us/azure/azure-monitor/app/live-stream>.

Metrics are based on traces. See attribute mapping below for how traces are mapped to requests,
dependencies and exceptions and how they are deemed "successful" or not.

To configure role, instance, and machine name provide `service.name`, `service.instance.id`, and
`host.name` resource attributes respectively in the resource.

Sample telemetry is not supported, yet.

```no_run
use opentelemetry::{global, trace::Tracer};
use opentelemetry_sdk::trace::SdkTracerProvider;

#[tokio::main]
async fn main() {
    let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING").unwrap();
    let exporter = opentelemetry_application_insights::Exporter::new_from_connection_string(
        connection_string,
        reqwest::blocking::Client::new(),
    )
    .expect("valid connection string");
    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor::builder(exporter.clone(), opentelemetry_sdk::runtime::Tokio).build())
        .with_span_processor(opentelemetry_application_insights::LiveMetricsSpanProcessor::new(exporter, opentelemetry_sdk::runtime::Tokio))
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    // ... send traces ...

    tracer_provider.shutdown().unwrap();
}
```
"#
)]
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
//! [`opentelemetry-http`]: https://crates.io/crates/opentelemetry-http
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`tokio`]: https://crates.io/crates/tokio
//!
//! Alternatively you can bring any other HTTP client by implementing the `HttpClient` trait.
//!
//! Map async/sync clients with the appropriate builder methods:
//!
//! - Sync clients with `{SdkTracerProvider,SdkLoggerProvider}.with_batch_exporter`/`PeriodicReader::builder`. If you're already in an
//! async context when creating the client, you might need to create it using
//! `std::thread::spawn(reqwest::blocking::Client::new).join().unwrap()`.
//! - Async clients with the corresponding experimental async APIs. _Or_ with the pipeline API and
//! `build_batch`/`install_batch`.
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
//! | `db.system.name`                                                           | Dependency Type                                          |
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
//! | `db.system.name`            | `db.system`                                |
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
#![doc(html_root_url = "https://docs.rs/opentelemetry-application-insights/0.42.0")]
#![allow(clippy::needless_doctest_main)]
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, deny(warnings))]
#![cfg_attr(test, allow(deprecated))]

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
pub use opentelemetry_http::HttpClient;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::ExportError;
#[cfg(any(feature = "trace", feature = "logs"))]
use opentelemetry_sdk::Resource;
#[cfg(feature = "live-metrics")]
pub use quick_pulse::{CollectorType, LiveMetricsSpanProcessor};
use std::{
    convert::TryInto,
    error::Error as StdError,
    fmt::Debug,
    sync::{Arc, Mutex},
    time::Duration,
};
#[cfg(feature = "live-metrics")]
use uploader_quick_pulse::PostOrPing;

/// Application Insights span exporter
#[derive(Clone)]
pub struct Exporter<C> {
    client: Arc<C>,
    track_endpoint: Arc<http::Uri>,
    #[cfg(feature = "live-metrics")]
    live_post_endpoint: http::Uri,
    #[cfg(feature = "live-metrics")]
    live_ping_endpoint: http::Uri,
    instrumentation_key: String,
    retry_notify: Option<Arc<Mutex<dyn FnMut(&Error, Duration) + Send + 'static>>>,
    #[cfg(feature = "trace")]
    sample_rate: f64,
    #[cfg(any(feature = "trace", feature = "logs"))]
    resource: Resource,
    #[cfg(any(feature = "trace", feature = "logs"))]
    resource_attributes_in_events_and_logs: bool,
}

impl<C: Debug> Debug for Exporter<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("Exporter");
        debug
            .field("client", &self.client)
            .field("track_endpoint", &self.track_endpoint)
            .field("instrumentation_key", &self.instrumentation_key);
        #[cfg(feature = "trace")]
        debug.field("sample_rate", &self.sample_rate);
        #[cfg(any(feature = "trace", feature = "logs"))]
        debug.field("resource", &self.resource).field(
            "resource_attributes_in_events_and_logs",
            &self.resource_attributes_in_events_and_logs,
        );
        #[cfg(feature = "live-metrics")]
        debug
            .field("live_post_endpoint", &self.live_post_endpoint)
            .field("live_ping_endpoint", &self.live_ping_endpoint);
        debug.finish()
    }
}

impl<C> Exporter<C> {
    /// Create a new exporter.
    #[deprecated(since = "0.27.0", note = "use new_from_connection_string() instead")]
    pub fn new(instrumentation_key: String, client: C) -> Self {
        Self {
            client: Arc::new(client),
            track_endpoint: Arc::new(append_v2_track(DEFAULT_BREEZE_ENDPOINT)),
            #[cfg(feature = "live-metrics")]
            live_post_endpoint: append_quick_pulse(
                DEFAULT_LIVE_ENDPOINT,
                PostOrPing::Post,
                &instrumentation_key,
            ),
            #[cfg(feature = "live-metrics")]
            live_ping_endpoint: append_quick_pulse(
                DEFAULT_LIVE_ENDPOINT,
                PostOrPing::Ping,
                &instrumentation_key,
            ),
            instrumentation_key,
            retry_notify: None,
            #[cfg(feature = "trace")]
            sample_rate: 100.0,
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource: Resource::builder_empty().build(),
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource_attributes_in_events_and_logs: false,
        }
    }

    /// Create a new exporter.
    ///
    /// Reads connection string from `APPLICATIONINSIGHTS_CONNECTION_STRING` environment variable.
    pub fn new_from_env(client: C) -> Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        let connection_string = std::env::var("APPLICATIONINSIGHTS_CONNECTION_STRING")?;
        Self::new_from_connection_string(connection_string, client)
    }

    /// Create a new exporter.
    pub fn new_from_connection_string(
        connection_string: impl AsRef<str>,
        client: C,
    ) -> Result<Self, Box<dyn StdError + Send + Sync + 'static>> {
        let connection_string: ConnectionString = connection_string.as_ref().parse()?;
        Ok(Self {
            client: Arc::new(client),
            track_endpoint: Arc::new(append_v2_track(&connection_string.ingestion_endpoint)),
            #[cfg(feature = "live-metrics")]
            live_post_endpoint: append_quick_pulse(
                &connection_string.live_endpoint,
                PostOrPing::Post,
                &connection_string.instrumentation_key,
            ),
            #[cfg(feature = "live-metrics")]
            live_ping_endpoint: append_quick_pulse(
                &connection_string.live_endpoint,
                PostOrPing::Ping,
                &connection_string.instrumentation_key,
            ),
            instrumentation_key: connection_string.instrumentation_key,
            retry_notify: None,
            #[cfg(feature = "trace")]
            sample_rate: 100.0,
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource: Resource::builder_empty().build(),
            #[cfg(any(feature = "trace", feature = "logs"))]
            resource_attributes_in_events_and_logs: false,
        })
    }

    /// Set a retry notification function that is called when a request to upload telemetry to
    /// Application Insights failed and will be retried.
    pub fn with_retry_notify<N>(mut self, retry_notify: N) -> Self
    where
        N: FnMut(&Error, Duration) + Send + 'static,
    {
        self.retry_notify = Some(Arc::new(Mutex::new(retry_notify)));
        self
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
        self.track_endpoint = Arc::new(append_v2_track(endpoint));
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

    /// Set whether resource attributes should be included in events.
    ///
    /// This affects both trace events and logs.
    ///
    /// Default: false.
    #[cfg(any(feature = "trace", feature = "logs"))]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "trace", feature = "logs"))))]
    pub fn with_resource_attributes_in_events_and_logs(
        mut self,
        resource_attributes_in_events_and_logs: bool,
    ) -> Self {
        self.resource_attributes_in_events_and_logs = resource_attributes_in_events_and_logs;
        self
    }
}

fn append_v2_track(uri: impl ToString) -> http::Uri {
    append_path(uri, "v2/track").expect("appending /v2/track should always work")
}

#[cfg(feature = "live-metrics")]
fn append_quick_pulse(
    uri: impl ToString,
    post_or_ping: PostOrPing,
    instrumentation_key: &str,
) -> http::Uri {
    let path = format!(
        "QuickPulseService.svc/{}?ikey={}",
        post_or_ping, instrumentation_key,
    );
    append_path(uri, &path).unwrap_or_else(|_| panic!("appending {} should always work", path))
}

fn append_path(
    uri: impl ToString,
    path: impl AsRef<str>,
) -> Result<http::Uri, http::uri::InvalidUri> {
    let mut curr = uri.to_string();
    if !curr.ends_with('/') {
        curr.push('/');
    }
    curr.push_str(path.as_ref());
    curr.try_into()
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

impl From<Error> for OTelSdkError {
    fn from(value: Error) -> Self {
        OTelSdkError::InternalFailure(value.to_string())
    }
}
