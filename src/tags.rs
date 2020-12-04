use crate::models::context_tag_keys::{self as tags, Tags};
use crate::{
    convert::{span_id_to_string, trace_id_to_string},
    models::context_tag_keys::TAG_KEY_LOOKUP,
};
use opentelemetry::{
    exporter::trace::SpanData,
    trace::{SpanId, SpanKind},
};
use opentelemetry_semantic_conventions as semcov;

pub(crate) fn get_tags_for_span(span: &SpanData) -> Tags {
    let mut map = Tags::new();

    // First, allow the user to explicitly express tags with attributes that start with `ai.`
    // These attributes do not collide with any opentelemetry semantic conventions, so it is
    // assumed that the user intends for them to be a part of the `tags` portion of the envelope.
    let ai_tags_iter = span
        .attributes
        .iter()
        .filter(|a| a.0.as_str().starts_with("ai."));
    for ai_tag in ai_tags_iter {
        if let Some(ctk) = TAG_KEY_LOOKUP.get(ai_tag.0.as_str()) {
            map.insert(ctk.clone(), ai_tag.1.to_string());
        }
    }

    // Set the operation id and operation parent id.
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

    // Ensure the name of the operation is `METHOD /the/route/path`.
    if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
        if let Some(method) = span.attributes.get(&semcov::trace::HTTP_METHOD) {
            if let Some(route) = span.attributes.get(&semcov::trace::HTTP_ROUTE) {
                map.insert(
                    tags::OPERATION_NAME,
                    format!("{} {}", method.as_str(), route.as_str()),
                );
            }
        }
    }

    if let Some(user_id) = span.attributes.get(&semcov::trace::ENDUSER_ID) {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id)
        // fit better?
        map.insert(tags::USER_AUTH_USER_ID, user_id.as_str().into_owned());
    }

    if let Some(service_name) = span.attributes.get(&semcov::resource::SERVICE_NAME) {
        let mut cloud_role = service_name.as_str().into_owned();
        if let Some(service_namespace) = span.attributes.get(&semcov::resource::SERVICE_NAMESPACE) {
            cloud_role.insert(0, '.');
            cloud_role.insert_str(0, &service_namespace.as_str());
        }

        map.insert(tags::CLOUD_ROLE, cloud_role);
    }

    if let Some(service_instance) = span.attributes.get(&semcov::resource::SERVICE_INSTANCE_ID) {
        map.insert(
            tags::CLOUD_ROLE_INSTANCE,
            service_instance.as_str().into_owned(),
        );
    }

    if let Some(service_version) = span.attributes.get(&semcov::resource::SERVICE_VERSION) {
        map.insert(
            tags::APPLICATION_VERSION,
            service_version.as_str().into_owned(),
        );
    }

    if let Some(sdk_name) = span.attributes.get(&semcov::resource::TELEMETRY_SDK_NAME) {
        let sdk_version = span
            .attributes
            .get(&semcov::resource::TELEMETRY_SDK_VERSION)
            .map(|v| v.as_str())
            .unwrap_or_else(|| "0.0.0".into());
        map.insert(
            tags::INTERNAL_SDK_VERSION,
            format!("{}:{}", sdk_name.as_str(), sdk_version),
        );
    }

    map
}

pub(crate) fn get_tags_for_event(span: &SpanData) -> Tags {
    let mut map = Tags::new();
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
