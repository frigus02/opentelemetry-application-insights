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
use contracts::{Base, Data, Envelope, RemoteDependencyData};
use opentelemetry::api::trace::span::SpanKind;
use opentelemetry::api::trace::span_context::{SpanId, TraceId};
use opentelemetry::api::{Key, Value};
use opentelemetry::exporter::trace;
use opentelemetry::sdk::trace::evicted_hash_map::EvictedHashMap;
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
        //match self.uploader.lock() {
        //    Ok(mut uploader) => {
        //        let jaeger_spans = batch.into_iter().map(Into::into).collect();
        //        uploader.upload(jaeger::Batch::new(self.process.clone(), jaeger_spans))
        //    }
        //    Err(_) => trace::ExportResult::FailedNotRetryable,
        //}
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
    // ai.operation.name
    map.insert(
        "ai.operation.parentId".into(),
        span_id_to_string(span.parent_span_id),
    );
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

const ATTR_DEPENDENCY_RESULT_CODE: &str = "dependency.result_code";
const ATTR_DEPENDENCY_SUCCESS: &str = "dependency.success";
const ATTR_DEPENDENCY_DATA: &str = "dependency.data";
const ATTR_DEPENDENCY_TARGET: &str = "dependency.target";
const ATTR_DEPENDENCY_TYPE: &str = "dependency.type";

struct DependencyAttributes {
    result_code: Option<String>,
    success: Option<bool>,
    data: Option<String>,
    target: Option<String>,
    type_: Option<String>,
    properties: Option<BTreeMap<String, String>>,
}

fn extract_dependency_attrs(attrs: &EvictedHashMap) -> DependencyAttributes {
    let mut result_code = None;
    let mut success = None;
    let mut data = None;
    let mut target = None;
    let mut type_ = None;
    let mut properties = BTreeMap::new();
    for (key, value) in attrs.iter() {
        if key == &Key::new(ATTR_DEPENDENCY_RESULT_CODE) {
            result_code = Some(value_to_string(value));
        } else if key == &Key::new(ATTR_DEPENDENCY_SUCCESS) {
            success = Some(match value {
                Value::Bool(v) => v.to_owned(),
                _ => false,
            });
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
        success,
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

fn new_envelope(
    span: Arc<trace::SpanData>,
    name: String,
    instrumentation_key: String,
    data: Base,
) -> Envelope {
    Envelope {
        name,
        time: DateTime::<Utc>::from(span.start_time).to_rfc3339_opts(SecondsFormat::Millis, true),
        i_key: Some(instrumentation_key),
        tags: Some(extract_tags(&span)),
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
                name: span.name.clone(),
                id: Some(span_id_to_string(span.span_context.span_id())),
                result_code: attrs.result_code,
                duration: duration_to_string(
                    span.end_time
                        .duration_since(span.start_time)
                        .expect("start time should be before end time"),
                ),
                success: attrs.success,
                data: attrs.data,
                target: attrs.target,
                type_: attrs.type_,
                properties: attrs.properties,
                ..RemoteDependencyData::default()
            }));
            new_envelope(
                span,
                "Microsoft.ApplicationInsights.RemoteDependency".into(),
                instrumentation_key,
                data,
            )
        }
        SpanKind::Server | SpanKind::Consumer => Envelope {
            // request, page view
            ..Envelope::default()
        },
        SpanKind::Internal => Envelope {
            //trace
            ..Envelope::default()
        },
    });

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
