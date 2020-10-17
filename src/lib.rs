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
//! spans:
//!
//! ```no_run
//! use opentelemetry::{api::trace::Tracer as _, sdk::trace::Tracer};
//! use opentelemetry_application_insights::Uninstall;
//!
//! fn init_tracer() -> (Tracer, Uninstall)  {
//!     let instrumentation_key = "...".to_string();
//!     opentelemetry_application_insights::new_pipeline(instrumentation_key)
//!         .install()
//! }
//!
//! fn main() {
//!     let (tracer, _uninstall) = init_tracer();
//!     tracer.in_span("main", |_cx| {});
//! }
//! ```
//!
//! The functions `build` and `install` automatically configure an asynchronous batch exporter if
//! you enable either the `async-std` or `tokio` feature for the `opentelemetry` crate. Otherwise
//! spans will be exported synchronously.
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
//! The Span's status determines the Success field of a Dependency or Request. Success is `true` if
//! the status is `OK`; otherwise `false`.
//!
//! For `INTERNAL` Spans the Dependency Type is always `"InProc"` and Success is `true`.
//!
//! The following of the Span's attributes map to special fields in Application Insights (the
//! mapping tries to follow the OpenTelemetry semantic conventions for [trace] and [resource]).
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
#![doc(html_root_url = "https://docs.rs/opentelemetry-application-insights/0.4.0")]
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(test, deny(warnings))]

mod convert;
mod http_client;
mod models;
mod tags;
mod uploader;

use async_trait::async_trait;
use convert::{
    attrs_to_properties, collect_attrs, duration_to_string, span_id_to_string, time_to_string,
};
pub use http_client::HttpClient;
use models::{
    Data, Envelope, ExceptionData, ExceptionDetails, MessageData, RemoteDependencyData,
    RequestData, Sanitize,
};
use opentelemetry::api::{
    trace::{Event, SpanKind, StatusCode, TracerProvider},
    Value,
};
use opentelemetry::exporter::trace::{ExportResult, SpanData, SpanExporter};
use opentelemetry::global;
use opentelemetry::sdk;
use std::collections::{BTreeMap, HashMap};
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
    /// ```
    /// let sample_rate = 0.3;
    /// let (tracer, _uninstall) = opentelemetry_application_insights::new_pipeline("...".into())
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
    /// ```
    /// # use opentelemetry::{api::KeyValue, sdk};
    /// # use std::sync::Arc;
    /// let (tracer, _uninstall) = opentelemetry_application_insights::new_pipeline("...".into())
    ///     .with_trace_config(sdk::trace::Config {
    ///         resource: Arc::new(sdk::Resource::new(vec![
    ///             KeyValue::new("service.name", "my-application"),
    ///         ])),
    ///         ..Default::default()
    ///     })
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
    /// Build a configured `sdk::Provider` with the recommended defaults.
    ///
    /// This will automatically configure an asynchronous batch exporter if you enable either the
    /// `async-std` or `tokio` feature for the `opentelemetry` crate. Otherwise spans will be
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
    /// This registers a global `sdk::Provider`. See the `build` function for details about how
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

/// Guard that uninstalls the Application Insights trace pipeline when dropped.
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
                let tags = get_tags_for_span(&span, &data.properties);
                (
                    Data::Request(data),
                    tags,
                    "Microsoft.ApplicationInsights.Request",
                )
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = (&span).into();
                let tags = get_tags_for_span(&span, &data.properties);
                (
                    Data::RemoteDependency(data),
                    tags,
                    "Microsoft.ApplicationInsights.RemoteDependency",
                )
            }
        };
        result.push(Envelope {
            name: name.into(),
            time: time_to_string(span.start_time),
            sample_rate: Some(self.sample_rate),
            i_key: Some(self.instrumentation_key.clone()),
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
                time: time_to_string(event.timestamp),
                sample_rate: Some(self.sample_rate),
                i_key: Some(self.instrumentation_key.clone()),
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
    async fn export(&self, batch: Vec<SpanData>) -> ExportResult {
        let mut envelopes: Vec<_> = batch
            .into_iter()
            .flat_map(|span| self.create_envelopes(span))
            .collect();
        for envelope in envelopes.iter_mut() {
            envelope.sanitize();
        }

        uploader::send(&self.client, envelopes).await
    }
}

impl From<&SpanData> for RequestData {
    fn from(span: &SpanData) -> RequestData {
        let mut data = RequestData {
            ver: 2,
            id: span_id_to_string(span.span_reference.span_id()),
            name: Some(span.name.clone()).filter(|x| !x.is_empty()),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            response_code: (span.status_code.clone() as i32).to_string(),
            success: span.status_code == StatusCode::OK,
            source: None,
            url: None,
            properties: None,
        };

        let attrs = collect_attrs(&span.attributes, span.resource.as_ref());

        if let Some(method) = attrs.get("http.method") {
            data.name = Some(if let Some(route) = attrs.get("http.route") {
                format!("{} {}", String::from(*method), String::from(*route))
            } else {
                String::from(*method)
            });
        }

        if let Some(status_code) = attrs.get("http.status_code") {
            data.response_code = String::from(*status_code);
        }

        if let Some(url) = attrs.get("http.url") {
            data.url = Some(String::from(*url));
        } else if let Some(target) = attrs.get("http.target") {
            let mut target = String::from(*target);
            if !target.starts_with('/') {
                target.insert(0, '/');
            }

            if let Some((scheme, host)) = opt_zip(attrs.get("http.scheme"), attrs.get("http.host"))
            {
                data.url = Some(format!(
                    "{}://{}{}",
                    String::from(*scheme),
                    String::from(*host),
                    target
                ));
            } else {
                data.url = Some(target);
            }
        }

        if let Some(client_ip) = attrs.get("http.client_ip") {
            data.source = Some(String::from(*client_ip));
        } else if let Some(peer_ip) = attrs.get("net.peer.ip") {
            data.source = Some(String::from(*peer_ip));
        }

        data.properties = attrs_to_properties(attrs);
        data
    }
}

impl From<&SpanData> for RemoteDependencyData {
    fn from(span: &SpanData) -> RemoteDependencyData {
        let mut data = RemoteDependencyData {
            ver: 2,
            id: Some(span_id_to_string(span.span_reference.span_id())),
            name: span.name.clone(),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            result_code: Some((span.status_code.clone() as i32).to_string()),
            success: Some(span.status_code == StatusCode::OK),
            data: None,
            target: None,
            type_: None,
            properties: None,
        };

        let attrs = collect_attrs(&span.attributes, span.resource.as_ref());

        if let Some(status_code) = attrs.get("http.status_code") {
            data.result_code = Some(String::from(*status_code));
        }

        if let Some(url) = attrs.get("http.url") {
            data.data = Some(String::from(*url));
        } else if let Some(statement) = attrs.get("db.statement") {
            data.data = Some(String::from(*statement));
        }

        if let Some(host) = attrs.get("http.host") {
            data.target = Some(String::from(*host));
        } else if let Some(peer_name) = attrs.get("net.peer.name") {
            if let Some(peer_port) = attrs.get("net.peer.port") {
                data.target = Some(format!(
                    "{}:{}",
                    String::from(*peer_name),
                    String::from(*peer_port)
                ));
            } else {
                data.target = Some(String::from(*peer_name));
            }
        } else if let Some(peer_ip) = attrs.get("net.peer.ip") {
            if let Some(peer_port) = attrs.get("net.peer.port") {
                data.target = Some(format!(
                    "{}:{}",
                    String::from(*peer_ip),
                    String::from(*peer_port)
                ));
            } else {
                data.target = Some(String::from(*peer_ip));
            }
        } else if let Some(db_name) = attrs.get("db.name") {
            data.target = Some(String::from(*db_name));
        }

        if span.span_kind == SpanKind::Internal {
            data.type_ = Some("InProc".into());
            data.success = Some(true);
        } else if let Some(db_system) = attrs.get("db.system") {
            data.type_ = Some(String::from(*db_system));
        } else if let Some(messaging_system) = attrs.get("messaging.system") {
            data.type_ = Some(String::from(*messaging_system));
        } else if let Some(rpc_system) = attrs.get("rpc.system") {
            data.type_ = Some(String::from(*rpc_system));
        } else if attrs.keys().any(|x| x.starts_with("http.")) {
            data.type_ = Some("HTTP".into());
        } else if attrs.keys().any(|x| x.starts_with("db.")) {
            data.type_ = Some("DB".into());
        }

        data.properties = attrs_to_properties(attrs);
        data
    }
}

impl From<&Event> for ExceptionData {
    fn from(event: &Event) -> ExceptionData {
        let mut attrs: HashMap<&str, &Value> = event
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), &kv.value))
            .collect();
        let exception = ExceptionDetails {
            type_name: attrs
                .remove("exception.type")
                .map(String::from)
                .unwrap_or_else(|| "<no type>".into()),
            message: attrs
                .remove("exception.message")
                .map(String::from)
                .unwrap_or_else(|| "<no message>".into()),
            stack: attrs.remove("exception.stacktrace").map(String::from),
        };
        ExceptionData {
            ver: 2,
            exceptions: vec![exception],
            properties: attrs_to_properties(attrs),
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
                event.name.clone()
            },
            properties: Some(
                event
                    .attributes
                    .iter()
                    .map(|kv| (kv.key.as_str().to_string(), String::from(&kv.value)))
                    .collect(),
            )
            .filter(|x: &BTreeMap<String, String>| !x.is_empty()),
        }
    }
}

fn opt_zip<T1, T2>(o1: Option<T1>, o2: Option<T2>) -> Option<(T1, T2)> {
    match (o1, o2) {
        (Some(v1), Some(v2)) => Some((v1, v2)),
        _ => None,
    }
}
