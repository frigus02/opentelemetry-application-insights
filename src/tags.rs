use crate::convert::{span_id_to_string, trace_id_to_string};
use crate::models::context_tag_keys::{self as tags, ContextTagKey};
use opentelemetry::{
    api::trace::{SpanId, SpanKind},
    exporter::trace::SpanData,
};
use opentelemetry_semantic_conventions as semcov;
use std::collections::BTreeMap;

pub(crate) fn get_tags_for_span(span: &SpanData) -> BTreeMap<ContextTagKey, String> {
    let mut map = BTreeMap::new();

    map.insert(
        tags::OPERATION_ID,
        trace_id_to_string(span.span_reference.trace_id()),
    );
    if span.parent_span_id != SpanId::invalid() {
        map.insert(
            tags::OPERATION_PARENT_ID,
            span_id_to_string(span.parent_span_id),
        );
    }

    if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
        if let Some(method) = span.attributes.get(&semcov::trace::HTTP_METHOD) {
            if let Some(route) = span.attributes.get(&semcov::trace::HTTP_ROUTE) {
                map.insert(
                    tags::OPERATION_NAME,
                    format!("{} {}", String::from(method), String::from(route)),
                );
            }
        }
    }

    if let Some(user_id) = span.attributes.get(&semcov::trace::ENDUSER_ID) {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id)
        // fit better?
        map.insert(tags::USER_AUTH_USER_ID, String::from(user_id));
    }

    if let Some(service_name) = span.attributes.get(&semcov::resource::SERVICE_NAME) {
        let mut cloud_role: String = service_name.into();
        if let Some(service_namespace) = span.attributes.get(&semcov::resource::SERVICE_NAMESPACE) {
            cloud_role.insert_str(0, ".");
            cloud_role.insert_str(0, &String::from(service_namespace));
        }

        map.insert(tags::CLOUD_ROLE, cloud_role);
    }

    if let Some(service_instance) = span.attributes.get(&semcov::resource::SERVICE_INSTANCE_ID) {
        map.insert(tags::CLOUD_ROLE_INSTANCE, String::from(service_instance));
    }

    if let Some(service_version) = span.attributes.get(&semcov::resource::SERVICE_VERSION) {
        map.insert(tags::APPLICATION_VERSION, String::from(service_version));
    }

    if let Some(sdk_name) = span.attributes.get(&semcov::resource::TELEMETRY_SDK_NAME) {
        let sdk_version = span
            .attributes
            .get(&semcov::resource::TELEMETRY_SDK_VERSION)
            .map(String::from)
            .unwrap_or_else(|| "0.0.0".into());
        map.insert(
            tags::INTERNAL_SDK_VERSION,
            format!("{}:{}", String::from(sdk_name), sdk_version),
        );
    }

    map
}

pub(crate) fn get_tags_for_event(span: &SpanData) -> BTreeMap<ContextTagKey, String> {
    let mut map = BTreeMap::new();
    map.insert(
        tags::OPERATION_ID,
        trace_id_to_string(span.span_reference.trace_id()),
    );
    map.insert(
        tags::OPERATION_PARENT_ID,
        span_id_to_string(span.span_reference.span_id()),
    );
    map
}
