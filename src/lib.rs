//! # OpenTelemetry Application Insights Exporter
//!
//! Collects OpenTelemetry spans and reports them to a Azure Application
//! Insights.
//!
//! ## Example
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
    /// Create a new exporter builder.
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

    /// Add specified application version to all telemetry items.
    pub fn with_application_version(mut self, ver: String) -> Self {
        self.common_tags.insert("ai.application.ver".into(), ver);
        self
    }

    /// Set sample rate, which is passed through to Application Insights. It
    /// should match the rate given to sampler.
    pub fn with_sample_rate(mut self, sample_rate: f64) -> Self {
        self.sample_rate = sample_rate;
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
                            .expect("start time should be before end time"),
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
                            .expect("start time should be before end time"),
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
