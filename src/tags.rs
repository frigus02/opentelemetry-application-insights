use crate::convert::{span_id_to_string, trace_id_to_string};
use crate::models::context_tag_keys::{self as tags, ContextTagKey};
use opentelemetry::api::{SpanId, SpanKind};
use opentelemetry::exporter::trace;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) fn get_common_tags() -> BTreeMap<ContextTagKey, String> {
    let mut tags = BTreeMap::new();
    tags.insert(
        tags::INTERNAL_SDK_VERSION,
        format!("rust:ot:ext{}", std::env!("CARGO_PKG_VERSION")),
    );
    tags
}

pub(crate) fn get_tags_for_span(
    span: &trace::SpanData,
    properties: &Option<BTreeMap<String, String>>,
) -> BTreeMap<ContextTagKey, String> {
    let mut map = BTreeMap::new();

    map.insert(
        tags::OPERATION_ID,
        trace_id_to_string(span.span_context.trace_id()),
    );
    if span.parent_span_id != SpanId::invalid() {
        map.insert(
            tags::OPERATION_PARENT_ID,
            span_id_to_string(span.parent_span_id),
        );
    }

    if let Some(properties) = properties {
        if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
            if let Some(method) = properties.get("http.method") {
                if let Some(route) = properties.get("http.route") {
                    map.insert(tags::OPERATION_NAME, format!("{} {}", method, route));
                }
            }
        }

        if let Some(user_id) = properties.get("enduser.id") {
            // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id)
            // fit better?
            map.insert(tags::USER_AUTH_USER_ID, user_id.to_owned());
        }

        if let Some(service_name) = properties.get("service.name") {
            let mut cloud_role: String = service_name.to_owned();
            if let Some(service_namespace) = properties.get("service.namespace") {
                cloud_role.insert_str(0, ".");
                cloud_role.insert_str(0, service_namespace);
            }

            map.insert(tags::CLOUD_ROLE, cloud_role);
        }

        if let Some(service_instance) = properties.get("service.instance.id") {
            map.insert(tags::CLOUD_ROLE_INSTANCE, service_instance.to_owned());
        }
    }

    map
}

pub(crate) fn get_tags_for_event(span: &Arc<trace::SpanData>) -> BTreeMap<ContextTagKey, String> {
    let mut map = BTreeMap::new();
    map.insert(
        tags::OPERATION_ID,
        trace_id_to_string(span.span_context.trace_id()),
    );
    map.insert(
        tags::OPERATION_PARENT_ID,
        span_id_to_string(span.span_context.span_id()),
    );
    map
}

pub(crate) fn merge_tags(
    common_tags: BTreeMap<ContextTagKey, String>,
    span_tags: BTreeMap<ContextTagKey, String>,
) -> BTreeMap<ContextTagKey, String> {
    common_tags.into_iter().chain(span_tags).collect()
}
