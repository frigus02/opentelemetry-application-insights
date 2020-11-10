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
//! spans (this example requires the **reqwest-client-blocking** feature):
//!
//! ```no_run
//! use opentelemetry::{trace::Tracer as _, sdk::trace::Tracer};
//! use opentelemetry_application_insights::Uninstall;
//!
//! fn init_tracer() -> (Tracer, Uninstall)  {
//!     let instrumentation_key = std::env::var("INSTRUMENTATION_KEY").unwrap();
//!     opentelemetry_application_insights::new_pipeline(instrumentation_key)
//!         .with_client(reqwest::blocking::Client::new())
//!         .install()
//! }
//!
//! fn main() {
//!     let (tracer, _uninstall) = init_tracer();
//!     tracer.in_span("main", |_cx| {});
//! }
//! ```
//!
//! ## Features
//!
//! The functions `build` and `install` automatically configure an asynchronous batch exporter if
//! you enable either the **async-std** or **tokio** feature for the `opentelemetry` crate.
//! Otherwise spans will be exported synchronously.
//!
//! In order to support different async runtimes, the exporter requires you to specify an HTTP
//! client that works with your chosen runtime. This crate comes with support for:
//!
//! - [`surf`] for [`async-std`]: enable the **surf-client** and **opentelemetry/async-std**
//!   features and configure the exporter with `with_client(surf::Client::new())`.
//! - [`reqwest`] for [`tokio`]: enable the **reqwest-client** and **opentelemetry/tokio** features
//!   and configure the exporter with `with_client(reqwest::Client::new())`.
//! - [`reqwest`] for synchronous exports: enable the **reqwest-blocking-client** feature and
//!   configure the exporter with `with_client(reqwest::blocking::Client::new())`.
//!
//! [`async-std`]: https://crates.io/crates/async-std
//! [`reqwest`]: https://crates.io/crates/reqwest
//! [`surf`]: https://crates.io/crates/surf
//! [`tokio`]: https://crates.io/crates/tokio
//!
//! Alternatively you can bring any other HTTP client by implementing the `HttpClient` trait.
//!
//! # Attribute mapping
//!
//! OpenTelemetry and Application Insights are using different terminology. This crate tries it's
//! best to map OpenTelemetry fields to their correct Application Insights pendant.
//!
//! - [OpenTelemetry specification: Span](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#span)
//! - [Application Insights data model](https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model)
//!
//! ## Spans
//!
//! The OpenTelemetry SpanKind determines the Application Insights telemetry type:
//!
//! | OpenTelemetry SpanKind           | Application Insights telemetry type |
//! | -------------------------------- | ----------------------------------- |
//! | `CLIENT`, `PRODUCER`, `INTERNAL` | Dependency                          |
//! | `SERVER`, `CONSUMER`             | Request                             |
//!
//! The Span's status determines the Success field of a Dependency or Request. Success is `false` if
//! the status `Error`; otherwise `true`.
//!
//! The following of the Span's attributes map to special fields in Application Insights (the
//! mapping tries to follow the OpenTelemetry semantic conventions for [trace] and [resource]).
//!
//! Note: for `INTERNAL` Spans the Dependency Type is always `"InProc"`.
//!
//! [trace]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions
//! [resource]: https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/resource/semantic_conventions
//!
//! | OpenTelemetry attribute key                    | Application Insights field     |
//! | ---------------------------------------------- | ------------------------------ |
//! | `service.version`                              | Context: Application version   |
//! | `enduser.id`                                   | Context: Authenticated user id |
//! | `service.namespace` + `service.name`           | Context: Cloud role            |
//! | `service.instance.id`                          | Context: Cloud role instance   |
//! | `telemetry.sdk.name` + `telemetry.sdk.version` | Context: Internal SDK version  |
//! | `http.url`                                     | Dependency Data                |
//! | `db.statement`                                 | Dependency Data                |
//! | `http.host`                                    | Dependency Target              |
//! | `net.peer.name` + `net.peer.port`              | Dependency Target              |
//! | `net.peer.ip` + `net.peer.port`                | Dependency Target              |
//! | `db.name`                                      | Dependency Target              |
//! | `http.status_code`                             | Dependency Result code         |
//! | `db.system`                                    | Dependency Type                |
//! | `messaging.system`                             | Dependency Type                |
//! | `rpc.system`                                   | Dependency Type                |
//! | `"HTTP"` if any `http.` attribute exists       | Dependency Type                |
//! | `"DB"` if any `db.` attribute exists           | Dependency Type                |
//! | `http.url`                                     | Request Url                    |
//! | `http.scheme` + `http.host` + `http.target`    | Request Url                    |
//! | `http.client_ip`                               | Request Source                 |
//! | `net.peer.ip`                                  | Request Source                 |
//! | `http.status_code`                             | Request Response code          |
//!
//! All other attributes are directly converted to custom properties.
//!
//! For Requests the attributes `http.method` and `http.route` override the Name.
//!
//! ## Events
//!
//! Events are converted into Exception telemetry if the event name equals `"exception"` (see
//! OpenTelemetry semantic conventions for [exceptions]) with the following mapping:
//!
//! | OpenTelemetry attribute key | Application Insights field |
//! | --------------------------- | -------------------------- |
//! | `exception.type`            | Exception type             |
//! | `exception.message`         | Exception message          |
//! | `exception.stacktrace`      | Exception call stack       |
//!
//! All other events are converted into Trace telemetry.
//!
//! All other attributes are directly converted to custom properties.
//!
//! [exceptions]: https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/semantic_conventions/exceptions.md
#![doc(html_root_url = "https://docs.rs/opentelemetry-application-insights/0.5.0")]
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(test, deny(warnings))]

mod convert;
mod http_client;
mod models;
mod tags;
mod uploader;

use async_trait::async_trait;
use convert::{attrs_to_properties, duration_to_string, span_id_to_string, time_to_string};
pub use http_client::HttpClient;
use models::{
    Data, Envelope, ExceptionData, ExceptionDetails, LimitedLenString1024, MessageData, Properties,
    RemoteDependencyData, RequestData,
};
use opentelemetry::{
    exporter::trace::{ExportResult, SpanData, SpanExporter},
    global, sdk,
    trace::{Event, SpanKind, StatusCode, TracerProvider},
    Key, Value,
};
use opentelemetry_semantic_conventions as semcov;
use std::collections::HashMap;
use tags::{get_tags_for_event, get_tags_for_span};

/// Create a new Application Insights exporter pipeline builder
pub fn new_pipeline(instrumentation_key: String) -> PipelineBuilder<()> {
    PipelineBuilder {
        client: (),
        config: None,
        instrumentation_key,
        sample_rate: 100.0,
    }
}

/// Application Insights exporter pipeline builder
#[derive(Debug)]
pub struct PipelineBuilder<C> {
    client: C,
    config: Option<sdk::trace::Config>,
    instrumentation_key: String,
    sample_rate: f64,
}

impl<C> PipelineBuilder<C> {
    /// Set HTTP client, which the exporter will use to send telemetry to Application Insights.
    ///
    /// Use this to set an HTTP client which fits your async runtime.
    pub fn with_client<NC>(self, client: NC) -> PipelineBuilder<NC> {
        PipelineBuilder {
            client,
            config: self.config,
            instrumentation_key: self.instrumentation_key,
            sample_rate: self.sample_rate,
        }
    }

    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    ///
    /// Note: This example requires [`reqwest`] and the **reqwest-client-blocking** feature.
    ///
    /// [`reqwest`]: https://crates.io/crates/reqwest
    ///
    /// ```no_run
    /// let sample_rate = 0.3;
    /// let (tracer, _uninstall) = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_sample_rate(sample_rate)
    ///     .install();
    /// ```
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = sample_rate * 100.0;
        self
    }

    /// Assign the SDK config for the exporter pipeline.
    ///
    /// Note: This example requires [`reqwest`] and the **reqwest-client-blocking** feature.
    ///
    /// [`reqwest`]: https://crates.io/crates/reqwest
    ///
    /// ```no_run
    /// # use opentelemetry::{KeyValue, sdk};
    /// let (tracer, _uninstall) = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_client(reqwest::blocking::Client::new())
    ///     .with_trace_config(sdk::trace::Config::default().with_resource(
    ///         sdk::Resource::new(vec![
    ///             KeyValue::new("service.name", "my-application"),
    ///         ]),
    ///     ))
    ///     .install();
    /// ```
    pub fn with_trace_config(self, config: sdk::trace::Config) -> Self {
        PipelineBuilder {
            config: Some(config),
            ..self
        }
    }
}

impl<C> PipelineBuilder<C>
where
    C: HttpClient + 'static,
{
    /// Build a configured `TracerProvider` with the recommended defaults.
    ///
    /// This will automatically configure an asynchronous batch exporter if you enable either the
    /// **async-std** or **tokio** feature for the `opentelemetry` crate. Otherwise spans will be
    /// exported synchronously.
    pub fn build(mut self) -> sdk::trace::TracerProvider {
        let config = self.config.take();
        let exporter =
            Exporter::new(self.instrumentation_key, self.client).with_sample_rate(self.sample_rate);

        let mut builder = sdk::trace::TracerProvider::builder().with_exporter(exporter);
        if let Some(config) = config {
            builder = builder.with_config(config);
        }

        builder.build()
    }

    /// Install an Application Insights pipeline with the recommended defaults.
    ///
    /// This registers a global `TracerProvider`. See the `build` function for details about how
    /// this provider is configured.
    pub fn install(self) -> (sdk::trace::Tracer, Uninstall) {
        let trace_provider = self.build();
        let tracer = trace_provider.get_tracer(
            "opentelemetry-application-insights",
            Some(env!("CARGO_PKG_VERSION")),
        );

        let provider_guard = global::set_tracer_provider(trace_provider);

        (tracer, Uninstall(provider_guard))
    }
}

/// Guard that uninstalls the Application Insights trace pipeline when dropped
#[derive(Debug)]
pub struct Uninstall(global::TracerProviderGuard);

/// Application Insights span exporter
#[derive(Debug)]
pub struct Exporter<C> {
    client: C,
    instrumentation_key: String,
    sample_rate: f64,
}

impl<C> Exporter<C> {
    /// Create a new exporter.
    pub fn new(instrumentation_key: String, client: C) -> Self {
        Self {
            client,
            instrumentation_key,
            sample_rate: 100.0,
        }
    }

    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = sample_rate * 100.0;
        self
    }

    fn create_envelopes(&self, span: SpanData) -> Vec<Envelope> {
        let mut result = Vec::with_capacity(1 + span.message_events.len());

        let (data, tags, name) = match span.span_kind {
            SpanKind::Server | SpanKind::Consumer => {
                let data: RequestData = (&span).into();
                let tags = get_tags_for_span(&span);
                (
                    Data::Request(data),
                    tags,
                    "Microsoft.ApplicationInsights.Request",
                )
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = (&span).into();
                let tags = get_tags_for_span(&span);
                (
                    Data::RemoteDependency(data),
                    tags,
                    "Microsoft.ApplicationInsights.RemoteDependency",
                )
            }
        };
        result.push(Envelope {
            name: name.into(),
            time: time_to_string(span.start_time).into(),
            sample_rate: Some(self.sample_rate),
            i_key: Some(self.instrumentation_key.clone().into()),
            tags: Some(tags),
            data: Some(data),
        });

        for event in span.message_events.iter() {
            let (data, name) = match event.name.as_ref() {
                "exception" => (
                    Data::Exception(event.into()),
                    "Microsoft.ApplicationInsights.Exception",
                ),
                _ => (
                    Data::Message(event.into()),
                    "Microsoft.ApplicationInsights.Message",
                ),
            };
            result.push(Envelope {
                name: name.into(),
                time: time_to_string(event.timestamp).into(),
                sample_rate: Some(self.sample_rate),
                i_key: Some(self.instrumentation_key.clone().into()),
                tags: Some(get_tags_for_event(&span)),
                data: Some(data),
            });
        }

        result
    }
}

#[async_trait]
impl<C> SpanExporter for Exporter<C>
where
    C: HttpClient,
{
    /// Export spans to Application Insights
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let envelopes: Vec<_> = batch
            .into_iter()
            .flat_map(|span| self.create_envelopes(span))
            .collect();

        uploader::send(&self.client, envelopes).await
    }
}

impl From<&SpanData> for RequestData {
    fn from(span: &SpanData) -> RequestData {
        let mut data = RequestData {
            ver: 2,
            id: span_id_to_string(span.span_context.span_id()).into(),
            name: Some(LimitedLenString1024::from(span.name.clone()))
                .filter(|x| !x.as_ref().is_empty()),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            response_code: (span.status_code.clone() as i32).to_string().into(),
            success: span.status_code != StatusCode::Error,
            source: None,
            url: None,
            properties: attrs_to_properties(&span.attributes, span.resource.as_ref()),
        };

        if let Some(method) = span.attributes.get(&semcov::trace::HTTP_METHOD) {
            data.name = Some(
                if let Some(route) = span.attributes.get(&semcov::trace::HTTP_ROUTE) {
                    format!("{} {}", method.as_str(), route.as_str()).into()
                } else {
                    method.into()
                },
            );
        }

        if let Some(status_code) = span.attributes.get(&semcov::trace::HTTP_STATUS_CODE) {
            data.response_code = status_code.into();
        }

        if let Some(url) = span.attributes.get(&semcov::trace::HTTP_URL) {
            data.url = Some(url.into());
        } else if let Some(target) = span.attributes.get(&semcov::trace::HTTP_TARGET) {
            let mut target = target.as_str().into_owned();
            if !target.starts_with('/') {
                target.insert(0, '/');
            }

            if let (Some(scheme), Some(host)) = (
                span.attributes.get(&semcov::trace::HTTP_SCHEME),
                span.attributes.get(&semcov::trace::HTTP_HOST),
            ) {
                data.url =
                    Some(format!("{}://{}{}", scheme.as_str(), host.as_str(), target).into());
            } else {
                data.url = Some(target.into());
            }
        }

        if let Some(client_ip) = span.attributes.get(&semcov::trace::HTTP_CLIENT_IP) {
            data.source = Some(client_ip.into());
        } else if let Some(peer_ip) = span.attributes.get(&semcov::trace::NET_PEER_IP) {
            data.source = Some(peer_ip.into());
        }

        data
    }
}

impl From<&SpanData> for RemoteDependencyData {
    fn from(span: &SpanData) -> RemoteDependencyData {
        let mut data = RemoteDependencyData {
            ver: 2,
            id: Some(span_id_to_string(span.span_context.span_id()).into()),
            name: span.name.clone().into(),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            result_code: Some((span.status_code.clone() as i32).to_string().into()),
            success: match span.status_code {
                StatusCode::Unset => None,
                StatusCode::Ok => Some(true),
                StatusCode::Error => Some(false),
            },
            data: None,
            target: None,
            type_: None,
            properties: attrs_to_properties(&span.attributes, span.resource.as_ref()),
        };

        if let Some(status_code) = span.attributes.get(&semcov::trace::HTTP_STATUS_CODE) {
            data.result_code = Some(status_code.into());
        }

        if let Some(url) = span.attributes.get(&semcov::trace::HTTP_URL) {
            data.data = Some(url.into());
        } else if let Some(statement) = span.attributes.get(&semcov::trace::DB_STATEMENT) {
            data.data = Some(statement.into());
        }

        if let Some(host) = span.attributes.get(&semcov::trace::HTTP_HOST) {
            data.target = Some(host.into());
        } else if let Some(peer_name) = span.attributes.get(&semcov::trace::NET_PEER_NAME) {
            if let Some(peer_port) = span.attributes.get(&semcov::trace::NET_PEER_PORT) {
                data.target = Some(format!("{}:{}", peer_name.as_str(), peer_port.as_str()).into());
            } else {
                data.target = Some(peer_name.into());
            }
        } else if let Some(peer_ip) = span.attributes.get(&semcov::trace::NET_PEER_IP) {
            if let Some(peer_port) = span.attributes.get(&semcov::trace::NET_PEER_PORT) {
                data.target = Some(format!("{}:{}", peer_ip.as_str(), peer_port.as_str()).into());
            } else {
                data.target = Some(peer_ip.into());
            }
        } else if let Some(db_name) = span.attributes.get(&semcov::trace::DB_NAME) {
            data.target = Some(db_name.into());
        }

        if span.span_kind == SpanKind::Internal {
            data.type_ = Some("InProc".into());
        } else if let Some(db_system) = span.attributes.get(&semcov::trace::DB_SYSTEM) {
            data.type_ = Some(db_system.into());
        } else if let Some(messaging_system) = span.attributes.get(&semcov::trace::MESSAGING_SYSTEM)
        {
            data.type_ = Some(messaging_system.into());
        } else if let Some(rpc_system) = span.attributes.get(&semcov::trace::RPC_SYSTEM) {
            data.type_ = Some(rpc_system.into());
        } else if let Some(ref properties) = data.properties {
            if properties.keys().any(|x| x.as_ref().starts_with("http.")) {
                data.type_ = Some("HTTP".into());
            } else if properties.keys().any(|x| x.as_ref().starts_with("db.")) {
                data.type_ = Some("DB".into());
            }
        }

        data
    }
}

impl From<&Event> for ExceptionData {
    fn from(event: &Event) -> ExceptionData {
        let mut attrs: HashMap<&Key, &Value> = event
            .attributes
            .iter()
            .map(|kv| (&kv.key, &kv.value))
            .collect();
        let exception = ExceptionDetails {
            type_name: attrs
                .remove(&semcov::trace::EXCEPTION_TYPE)
                .map(Into::into)
                .unwrap_or_else(|| "<no type>".into()),
            message: attrs
                .remove(&semcov::trace::EXCEPTION_MESSAGE)
                .map(Into::into)
                .unwrap_or_else(|| "<no message>".into()),
            stack: attrs
                .remove(&semcov::trace::EXCEPTION_STACKTRACE)
                .map(Into::into),
        };
        ExceptionData {
            ver: 2,
            exceptions: vec![exception],
            properties: Some(
                attrs
                    .iter()
                    .map(|(k, v)| (k.as_str().into(), (*v).into()))
                    .collect(),
            )
            .filter(|x: &Properties| !x.is_empty()),
        }
    }
}

impl From<&Event> for MessageData {
    fn from(event: &Event) -> MessageData {
        MessageData {
            ver: 2,
            message: if event.name.is_empty() {
                "<no message>".into()
            } else {
                event.name.clone().into()
            },
            properties: Some(
                event
                    .attributes
                    .iter()
                    .map(|kv| (kv.key.as_str().into(), (&kv.value).into()))
                    .collect(),
            )
            .filter(|x: &Properties| !x.is_empty()),
        }
    }
}
