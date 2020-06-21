//! An [Azure Application Insights](https://docs.microsoft.com/en-us/azure/azure-monitor/app/app-insights-overview) exporter implementation for [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust).
//!
//! **Disclaimer**: This is not an official Microsoft product.
//!
//! # Usage
//!
//! Configure the exporter:
//!
//! ```rust,no_run
//! use opentelemetry::{global, sdk};
//!
//! fn init_tracer() {
//!     let instrumentation_key = "...".to_string();
//!     let exporter = opentelemetry_application_insights::Exporter::new(instrumentation_key);
//!     let provider = sdk::Provider::builder()
//!         .with_simple_exporter(exporter)
//!         .build();
//!     global::set_provider(provider);
//! }
//! ```
//!
//! Then follow the documentation of [opentelemetry](https://github.com/open-telemetry/opentelemetry-rust) to create spans and events.
//!
//! # Attribute mapping
//!
//! OpenTelemetry and Application Insights are using different terminology. This crate tries it's best to map OpenTelemetry fields to their correct Application Insights pendant.
//!
//! - [OpenTelemetry specification: Span](https://github.com/open-telemetry/opentelemetry-specification/blob/master/specification/trace/api.md#span)
//! - [Application Insights data model](https://docs.microsoft.com/en-us/azure/azure-monitor/app/data-model)
//!
//! The OpenTelemetry SpanKind determines the Application Insights telemetry type:
//!
//! | OpenTelemetry SpanKind           | Application Insights telemetry type |
//! | -------------------------------- | ----------------------------------- |
//! | `CLIENT`, `PRODUCER`             | Dependency                          |
//! | `SERVER`, `CONSUMER`, `INTERNAL` | Request                             |
//!
//! The Span's list of Events are converted to Trace telemetry.
//!
//! The Span's status determines the Success field of a Dependency or Request. Success is `true` if the status is `OK`; otherwise `false`.
//!
//! The following of the Span's attributes map to special fields in Application Insights (the mapping tries to follow [OpenTelemetry semantic conventions](https://github.com/open-telemetry/opentelemetry-specification/tree/master/specification/trace/semantic_conventions)).
//!
//! | OpenTelemetry attribute key              | Application Insights field     |
//! | ---------------------------------------- | ------------------------------ |
//! | `enduser.id`                             | Context: Authenticated user id |
//! | `net.host.name`                          | Context: Cloud role instance   |
//! | `http.url`                               | Dependency Data                |
//! | `db.statement`                           | Dependency Data                |
//! | `net.peer.ip`                            | Dependency Target              |
//! | `net.peer.name`                          | Dependency Target              |
//! | `http.host`                              | Dependency Target              |
//! | `http.status_code`                       | Dependency Result code         |
//! | `db.type`                                | Dependency Type                |
//! | `messaging.system`                       | Dependency Type                |
//! | `"HTTP"` if any `http.` attribute exists | Dependency Type                |
//! | `http.target`                            | Request Url                    |
//! | `http.url`                               | Request Url                    |
//! | `net.peer.ip`                            | Request Source                 |
//! | `http.status_code`                       | Request Response code          |
//!
//! All other attributes are be directly converted to custom properties.
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(test, deny(warnings))]

mod convert;
mod models;
mod tags;
mod uploader;

use convert::{duration_to_string, span_id_to_string, time_to_string, value_to_string};
use models::{Data, Envelope, MessageData, RemoteDependencyData, RequestData};
use opentelemetry::api::{KeyValue, SpanKind, StatusCode};
use opentelemetry::exporter::trace;
use opentelemetry::sdk::EvictedHashMap;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tags::{
    get_tag_key_from_attribute_key, get_tags_from_span, get_tags_from_span_for_event, merge_tags,
};

/// Application Insights span exporter
#[derive(Debug)]
pub struct Exporter {
    instrumentation_key: String,
    common_tags: BTreeMap<String, String>,
    sample_rate: f64,
}

impl Exporter {
    /// Create a new exporter.
    pub fn new(instrumentation_key: String) -> Self {
        let mut common_tags = BTreeMap::new();
        common_tags.insert(
            "ai.internal.sdkVersion".into(),
            format!(
                "{}:{}",
                std::env!("CARGO_PKG_NAME"),
                std::env!("CARGO_PKG_VERSION")
            ),
        );
        Self {
            instrumentation_key,
            common_tags,
            sample_rate: 100.0,
        }
    }

    /// Add an application version to all telemetry items.
    ///
    /// ```
    /// let exporter = opentelemetry_application_insights::Exporter::new("...".into())
    ///     .with_application_version(std::env!("CARGO_PKG_VERSION").into());
    /// ```
    pub fn with_application_version(mut self, ver: String) -> Self {
        self.common_tags.insert("ai.application.ver".into(), ver);
        self
    }

    /// Set sample rate, which is passed through to Application Insights. It should be a value
    /// between 0 and 1 and match the rate given to the sampler.
    ///
    /// Default: 1.0
    ///
    /// ```
    /// # use opentelemetry::{global, sdk};
    /// let sample_rate = 0.3;
    /// let exporter = opentelemetry_application_insights::Exporter::new("...".into())
    ///     .with_sample_rate(sample_rate);
    /// let provider = sdk::Provider::builder()
    ///     .with_simple_exporter(exporter)
    ///     .with_config(sdk::Config {
    ///         default_sampler: Box::new(sdk::Sampler::Probability(sample_rate)),
    ///         ..Default::default()
    ///     })
    ///     .build();
    /// ```
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        // Application Insights expects the sample rate as a percentage.
        self.sample_rate = sample_rate * 100.0;
        self
    }

    fn create_envelopes(&self, span: Arc<trace::SpanData>) -> Vec<Envelope> {
        let mut result = Vec::with_capacity(1 + span.message_events.len());
        result.push(match span.span_kind {
            SpanKind::Client | SpanKind::Producer => {
                let attrs = extract_dependency_attrs(&span.attributes);
                let data = Data::RemoteDependency(RemoteDependencyData {
                    ver: 2,
                    id: Some(span_id_to_string(span.span_context.span_id())),
                    name: span.name.clone(),
                    result_code: attrs.result_code,
                    duration: duration_to_string(
                        span.end_time
                            .duration_since(span.start_time)
                            .unwrap_or(Duration::from_secs(0)),
                    ),
                    success: Some(span.status_code == StatusCode::OK),
                    data: attrs.data,
                    target: attrs.target,
                    type_: attrs.type_,
                    properties: attrs.properties,
                });
                Envelope {
                    name: "Microsoft.ApplicationInsights.RemoteDependency".into(),
                    time: time_to_string(span.start_time),
                    sample_rate: Some(self.sample_rate),
                    i_key: Some(self.instrumentation_key.clone()),
                    tags: Some(merge_tags(
                        self.common_tags.clone(),
                        get_tags_from_span(&span),
                        attrs.tags,
                    )),
                    data: Some(data),
                }
            }
            SpanKind::Server | SpanKind::Consumer | SpanKind::Internal => {
                let attrs = extract_request_attrs(&span.attributes);
                let data = Data::Request(RequestData {
                    ver: 2,
                    id: span_id_to_string(span.span_context.span_id()),
                    source: attrs.source,
                    name: Some(span.name.clone()),
                    duration: duration_to_string(
                        span.end_time
                            .duration_since(span.start_time)
                            .unwrap_or(Duration::from_secs(0)),
                    ),
                    response_code: attrs.response_code,
                    success: span.status_code == StatusCode::OK,
                    url: attrs.url,
                    properties: attrs.properties,
                });
                Envelope {
                    name: "Microsoft.ApplicationInsights.Request".into(),
                    time: time_to_string(span.start_time),
                    sample_rate: Some(self.sample_rate),
                    i_key: Some(self.instrumentation_key.clone()),
                    tags: Some(merge_tags(
                        self.common_tags.clone(),
                        get_tags_from_span(&span),
                        attrs.tags,
                    )),
                    data: Some(data),
                }
            }
        });

        for event in span.message_events.iter() {
            let attrs = extract_trace_attrs(&event.attributes);
            let data = Data::Message(MessageData {
                ver: 2,
                message: event.name.clone(),
                properties: attrs.properties,
            });
            result.push(Envelope {
                name: "Microsoft.ApplicationInsights.Message".into(),
                time: time_to_string(event.timestamp),
                sample_rate: Some(self.sample_rate),
                i_key: Some(self.instrumentation_key.clone()),
                tags: Some(merge_tags(
                    self.common_tags.clone(),
                    get_tags_from_span_for_event(&span),
                    attrs.tags,
                )),
                data: Some(data),
            });
        }

        result
    }
}

impl trace::SpanExporter for Exporter {
    /// Export spans to Application Insights
    fn export(&self, batch: Vec<Arc<trace::SpanData>>) -> trace::ExportResult {
        let envelopes = batch
            .into_iter()
            .flat_map(|span| self.create_envelopes(span))
            .collect();
        uploader::send(envelopes).into()
    }

    fn shutdown(&self) {}
}

struct RequestAttributes {
    source: Option<String>,
    response_code: String,
    url: Option<String>,
    tags: BTreeMap<String, String>,
    properties: Option<BTreeMap<String, String>>,
}

fn extract_request_attrs(attrs: &EvictedHashMap) -> RequestAttributes {
    let mut source = None;
    let mut response_code = None;
    let mut url = None;
    let mut properties = BTreeMap::new();
    let mut tags = BTreeMap::new();
    for (key, value) in attrs.iter() {
        if key.as_str() == "net.peer.ip" {
            source = Some(value_to_string(value));
        } else if key.as_str() == "http.status_code" {
            response_code = Some(value_to_string(value));
        } else if key.as_str() == "http.target" || key.as_str() == "http.url" {
            url = Some(value_to_string(value));
        } else if let Some(tag) = get_tag_key_from_attribute_key(key) {
            tags.insert(tag, value_to_string(value));
        } else {
            properties.insert(key.as_str().to_string(), value_to_string(value));
        }
    }

    RequestAttributes {
        source,
        response_code: response_code.unwrap_or_else(|| "none".into()),
        url,
        tags,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    }
}

struct DependencyAttributes {
    result_code: Option<String>,
    data: Option<String>,
    target: Option<String>,
    type_: Option<String>,
    tags: BTreeMap<String, String>,
    properties: Option<BTreeMap<String, String>>,
}

fn extract_dependency_attrs(attrs: &EvictedHashMap) -> DependencyAttributes {
    let mut result_code = None;
    let mut data = None;
    let mut target = None;
    let mut type_ = None;
    let mut tags = BTreeMap::new();
    let mut properties = BTreeMap::new();
    let mut is_http = false;
    for (key, value) in attrs.iter() {
        if key.as_str().starts_with("http.") {
            is_http = true;
        }

        if key.as_str() == "http.status_code" {
            result_code = Some(value_to_string(value));
        } else if key.as_str() == "http.url" || key.as_str() == "db.statement" {
            data = Some(value_to_string(value));
        } else if key.as_str() == "net.peer.ip"
            || key.as_str() == "net.peer.name"
            || key.as_str() == "http.host"
        {
            target = Some(value_to_string(value));
        } else if key.as_str() == "db.type" || key.as_str() == "messaging.system" {
            type_ = Some(value_to_string(value));
        } else if let Some(tag) = get_tag_key_from_attribute_key(key) {
            tags.insert(tag, value_to_string(value));
        } else {
            properties.insert(key.as_str().to_string(), value_to_string(value));
        }
    }

    if type_.is_none() && is_http {
        type_ = Some("HTTP".into());
    }

    DependencyAttributes {
        result_code,
        data,
        target,
        type_,
        tags,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    }
}

struct TraceAttributes {
    tags: BTreeMap<String, String>,
    properties: Option<BTreeMap<String, String>>,
}

fn extract_trace_attrs(attrs: &[KeyValue]) -> TraceAttributes {
    let mut tags = BTreeMap::new();
    let mut properties = BTreeMap::new();
    for KeyValue { key, value } in attrs {
        if let Some(tag) = get_tag_key_from_attribute_key(key) {
            tags.insert(tag, value_to_string(value));
        } else {
            properties.insert(key.as_str().to_string(), value_to_string(value));
        }
    }

    TraceAttributes {
        tags,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    }
}

impl From<uploader::Response> for trace::ExportResult {
    fn from(response: uploader::Response) -> trace::ExportResult {
        match response {
            uploader::Response::Success => Self::Success,
            uploader::Response::Retry => Self::FailedRetryable,
            uploader::Response::NoRetry => Self::FailedNotRetryable,
        }
    }
}
