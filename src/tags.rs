use crate::convert::{span_id_to_string, trace_id_to_string, value_to_string};
use log::debug;
use opentelemetry::api::{SpanId, SpanKind, Value};
use opentelemetry::exporter::trace;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) fn get_common_tags() -> BTreeMap<String, String> {
    let mut tags = BTreeMap::new();
    tags.insert(
        "ai.internal.sdkVersion".into(),
        format!("rust:ot:ext{}", std::env!("CARGO_PKG_VERSION")),
    );
    tags.insert(
        "ai.cloud.role".into(),
        std::env::args()
            .next()
            .and_then(|process_name| {
                std::path::Path::new(&process_name)
                    .file_stem()
                    .and_then(|file_stem| file_stem.to_str())
                    .map(|file_stem| file_stem.to_string())
            })
            .unwrap_or_else(|| "Rust Application".into()),
    );
    match gethostname::gethostname().into_string() {
        Ok(hostname) => {
            tags.insert("ai.cloud.roleInstance".into(), hostname);
        }
        Err(_) => {
            debug!("Failed to read hostname. Cloud role instance tags will be be set.");
        }
    }
    tags
}

pub(crate) fn get_tags_for_span(
    span: &Arc<trace::SpanData>,
    attrs: &HashMap<&str, &Value>,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    map.insert(
        "ai.operation.id".into(),
        trace_id_to_string(span.span_context.trace_id()),
    );
    if span.parent_span_id != SpanId::invalid() {
        map.insert(
            "ai.operation.parentId".into(),
            span_id_to_string(span.parent_span_id),
        );
    }

    if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
        if let Some(method) = attrs.get("http.method") {
            if let Some(route) = attrs.get("http.route") {
                map.insert(
                    "ai.operation.name".into(),
                    format!("{} {}", value_to_string(method), value_to_string(route)),
                );
            }
        }
    }

    if let Some(user_id) = attrs.get("enduser.id") {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id) fit
        // better?
        map.insert("ai.user.authUserId".into(), value_to_string(user_id));
    }

    map
}

pub(crate) fn get_tags_for_event(span: &Arc<trace::SpanData>) -> BTreeMap<String, String> {
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

pub(crate) fn merge_tags(
    common_tags: BTreeMap<String, String>,
    span_tags: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    common_tags.into_iter().chain(span_tags).collect()
}
