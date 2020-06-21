use crate::convert::{span_id_to_string, trace_id_to_string};
use opentelemetry::api::{Key, SpanId, SpanKind};
use opentelemetry::exporter::trace;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) fn get_tags_from_span(span: &Arc<trace::SpanData>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
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
    map
}

pub(crate) fn get_tags_from_span_for_event(
    span: &Arc<trace::SpanData>,
) -> BTreeMap<String, String> {
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

pub(crate) fn get_tag_key_from_attribute_key(key: &Key) -> Option<String> {
    match key.as_str() {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id) fit
        // better?
        "enduser.id" => Some("ai.user.authUserId".into()),
        "net.host.name" => Some("ai.cloud.roleInstance".into()),
        _ => None,
    }
}

pub(crate) fn merge_tags(
    common_tags: BTreeMap<String, String>,
    span_tags: BTreeMap<String, String>,
    attr_tags: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    common_tags
        .into_iter()
        .chain(span_tags)
        .chain(attr_tags)
        .collect()
}
