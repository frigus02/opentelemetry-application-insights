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
//!     let exporter = opentelemetry_application_insights::Exporter::new("...");
//!     let provider = sdk::Provider::builder()
//!         .with_simple_exporter(exporter)
//!         .with_config(sdk::Config {
//!             default_sampler: Box::new(sdk::Sampler::AlwaysOn),
//!             ..Default::default()
//!         })
//!         .build();
//!     global::set_provider(provider);
//! }
//! ```
#![deny(missing_docs, unreachable_pub, missing_debug_implementations)]
#![cfg_attr(test, deny(warnings))]

mod contracts;
mod uploader;

use chrono::{DateTime, SecondsFormat, Utc};
use contracts::{Base, Data, Envelope, MessageData, RemoteDependencyData, RequestData};
use opentelemetry::api::{Event, Key, KeyValue, SpanId, SpanKind, StatusCode, TraceId, Value};
use opentelemetry::exporter::trace;
use opentelemetry::sdk::EvictedHashMap;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use uploader::Uploader;

/// Application Insights span exporter
#[derive(Debug)]
pub struct Exporter {
    instrumentation_key: String,
    uploader: Uploader,
}

impl Exporter {
    /// Create a new exporter builder.
    pub fn new(instrumentation_key: String) -> Self {
        Self {
            instrumentation_key,
            uploader: Uploader::new(),
        }
    }
}

impl trace::SpanExporter for Exporter {
    /// Export spans to Application Insights
    fn export(&self, batch: Vec<Arc<trace::SpanData>>) -> trace::ExportResult {
        let envelopes = batch
            .into_iter()
            .flat_map(|span| into_envelopes(span, self.instrumentation_key.clone()))
            .collect();
        self.uploader.send(envelopes).into()
    }

    fn shutdown(&self) {}
}

fn trace_id_to_string(trace_id: TraceId) -> String {
    format!("{:032x}", trace_id.to_u128())
}

fn span_id_to_string(span_id: SpanId) -> String {
    format!("{:016x}", span_id.to_u64())
}

fn duration_to_string(duration: Duration) -> String {
    let micros = duration.as_micros();
    let s = micros / 1_000_000 % 60;
    let m = micros / 1_000_000 / 60 % 60;
    let h = micros / 1_000_000 / 60 / 60 % 24;
    let d = micros / 1_000_000 / 60 / 60 / 24;
    let micros_remaining = micros / 1_000_000;
    format!(
        "{}.{:0>2}:{:0>2}:{:0>2}.{:0>7}",
        d, h, m, s, micros_remaining
    )
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Bool(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::String(v) => v.to_owned(),
        Value::Bytes(v) => base64::encode(&v),
    }
}

fn extract_tags(span: &Arc<trace::SpanData>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    // ai.application.ver
    // ai.device.id
    // ai.device.locale
    // ai.device.model
    // ai.device.oemName
    // ai.device.osVersion
    // ai.device.type
    // ai.location.ip
    // ai.location.country
    // ai.location.province
    // ai.location.city
    map.insert(
        "ai.operation.id".into(),
        trace_id_to_string(span.span_context.trace_id()),
    );
    if span.span_kind == SpanKind::Internal {
        map.insert("ai.operation.name".into(), "OPERATION".into());
    }
    if span.parent_span_id != SpanId::invalid() {
        map.insert(
            "ai.operation.parentId".into(),
            span_id_to_string(span.parent_span_id),
        );
    }
    // ai.operation.syntheticSource
    // ai.operation.correlationVector
    // ai.session.id
    // ai.session.isFirst
    // ai.user.accountId
    // ai.user.id
    // au.user.authUserId
    // ai.cloud.role
    // ai.cloud.roleVer
    // ai.cloud.roleInstance
    // ai.cloud.location
    // ai.internal.sdkVersion
    // ai.internal.agentVersion
    // ai.internal.nodeName
    map
}

fn extract_tags_for_event(span: &Arc<trace::SpanData>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert(
        "ai.operation.id".into(),
        trace_id_to_string(span.span_context.trace_id()),
    );
    map.insert(
        "ai.operation.parentId".into(),
        span_id_to_string(span.span_context.span_id()),
    );
    map
}

const ATTR_REQUEST_SOURCE: &str = "request.source";
const ATTR_REQUEST_RESPONSE_CODE: &str = "request.response_code";
const ATTR_REQUEST_URL: &str = "request.url";
const ATTR_DEPENDENCY_RESULT_CODE: &str = "dependency.result_code";
const ATTR_DEPENDENCY_DATA: &str = "dependency.data";
const ATTR_DEPENDENCY_TARGET: &str = "dependency.target";
const ATTR_DEPENDENCY_TYPE: &str = "dependency.type";

struct RequestAttributes {
    source: Option<String>,
    response_code: String,
    url: Option<String>,
    properties: Option<BTreeMap<String, String>>,
}

fn extract_request_attrs(attrs: &EvictedHashMap) -> RequestAttributes {
    let mut source = None;
    let mut response_code = None;
    let mut url = None;
    let mut properties = BTreeMap::new();
    for (key, value) in attrs.iter() {
        if key == &Key::new(ATTR_REQUEST_SOURCE) {
            source = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_REQUEST_RESPONSE_CODE) {
            response_code = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_REQUEST_URL) {
            url = Some(value_to_string(value));
        } else {
            properties.insert(key.as_str().to_string(), value_to_string(value));
        }
    }

    RequestAttributes {
        source,
        response_code: response_code.unwrap_or_else(|| "".into()),
        url,
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
    properties: Option<BTreeMap<String, String>>,
}

fn extract_dependency_attrs(attrs: &EvictedHashMap) -> DependencyAttributes {
    let mut result_code = None;
    let mut data = None;
    let mut target = None;
    let mut type_ = None;
    let mut properties = BTreeMap::new();
    for (key, value) in attrs.iter() {
        if key == &Key::new(ATTR_DEPENDENCY_RESULT_CODE) {
            result_code = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_DEPENDENCY_DATA) {
            data = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_DEPENDENCY_TARGET) {
            target = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_DEPENDENCY_TYPE) {
            type_ = Some(value_to_string(value));
        } else {
            properties.insert(key.as_str().to_string(), value_to_string(value));
        }
    }

    DependencyAttributes {
        result_code,
        data,
        target,
        type_,
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    }
}

struct TraceAttributes {
    properties: Option<BTreeMap<String, String>>,
}

fn extract_trace_attrs(attrs: &[KeyValue]) -> TraceAttributes {
    let mut properties = BTreeMap::new();
    for KeyValue { key, value } in attrs {
        properties.insert(key.as_str().to_string(), value_to_string(value));
    }

    TraceAttributes {
        properties: if properties.is_empty() {
            None
        } else {
            Some(properties)
        },
    }
}

fn new_envelope(
    span: &Arc<trace::SpanData>,
    name: String,
    instrumentation_key: String,
    data: Base,
) -> Envelope {
    Envelope {
        name,
        time: DateTime::<Utc>::from(span.start_time).to_rfc3339_opts(SecondsFormat::Millis, true),
        i_key: Some(instrumentation_key),
        tags: Some(extract_tags(span)),
        data: Some(data),
        ..Envelope::default()
    }
}

fn new_envelope_for_event(
    event: &Event,
    span: &Arc<trace::SpanData>,
    name: String,
    instrumentation_key: String,
    data: Base,
) -> Envelope {
    Envelope {
        name,
        time: DateTime::<Utc>::from(event.timestamp).to_rfc3339_opts(SecondsFormat::Millis, true),
        i_key: Some(instrumentation_key),
        tags: Some(extract_tags_for_event(span)),
        data: Some(data),
        ..Envelope::default()
    }
}

fn into_envelopes(span: Arc<trace::SpanData>, instrumentation_key: String) -> Vec<Envelope> {
    let mut result = Vec::with_capacity(1 + span.message_events.len());
    result.push(match span.span_kind {
        SpanKind::Client | SpanKind::Producer => {
            let attrs = extract_dependency_attrs(&span.attributes);
            let data = Base::Data(Data::RemoteDependencyData(RemoteDependencyData {
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
                ..RemoteDependencyData::default()
            }));
            new_envelope(
                &span,
                "Microsoft.ApplicationInsights.RemoteDependency".into(),
                instrumentation_key.clone(),
                data,
            )
        }
        SpanKind::Server | SpanKind::Consumer | SpanKind::Internal => {
            let attrs = extract_request_attrs(&span.attributes);
            let data = Base::Data(Data::RequestData(RequestData {
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
                ..RequestData::default()
            }));
            new_envelope(
                &span,
                "Microsoft.ApplicationInsights.Request".into(),
                instrumentation_key.clone(),
                data,
            )
        }
    });

    for event in span.message_events.iter() {
        let attrs = extract_trace_attrs(&event.attributes);
        let data = Base::Data(Data::MessageData(MessageData {
            message: event.name.clone(),
            properties: attrs.properties,
            ..MessageData::default()
        }));
        result.push(new_envelope_for_event(
            &event,
            &span,
            "Microsoft.ApplicationInsights.Message".into(),
            instrumentation_key.clone(),
            data,
        ));
    }

    result
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
