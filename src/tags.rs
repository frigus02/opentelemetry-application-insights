use crate::models::context_tag_keys::{self as tags, Tags, TAG_KEY_LOOKUP};
use opentelemetry::{
    sdk::export::trace::SpanData,
    trace::{SpanId, SpanKind},
    Key, Value,
};
#[cfg(feature = "metrics")]
use opentelemetry::{
    sdk::{AttributeSet, Resource},
    InstrumentationLibrary,
};
use opentelemetry_semantic_conventions as semcov;
use std::collections::HashMap;

pub(crate) fn get_tags_for_span(span: &SpanData) -> Tags {
    let mut tags = get_tags_from_attrs(span.resource.iter().chain(span.attributes.iter()));

    // Set the operation id and operation parent id.
    tags.insert(tags::OPERATION_ID, span.span_context.trace_id().to_string());
    if span.parent_span_id != SpanId::INVALID {
        tags.insert(tags::OPERATION_PARENT_ID, span.parent_span_id.to_string());
    }

    // Ensure the name of the operation is `METHOD /the/route/path`.
    if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
        if let Some(method) = span
            .attributes
            .get(&semcov::trace::HTTP_REQUEST_METHOD)
            .or_else(|| {
                span.attributes.get(
                    #[allow(deprecated)]
                    &semcov::trace::HTTP_METHOD,
                )
            })
        {
            if let Some(route) = span.attributes.get(&semcov::trace::HTTP_ROUTE) {
                tags.insert(
                    tags::OPERATION_NAME,
                    format!("{} {}", method.as_str(), route.as_str()),
                );
            }
        }
    }

    tags
}

pub(crate) fn get_tags_for_event(span: &SpanData) -> Tags {
    let mut tags = Tags::new();
    tags.insert(tags::OPERATION_ID, span.span_context.trace_id().to_string());
    tags.insert(
        tags::OPERATION_PARENT_ID,
        span.span_context.span_id().to_string(),
    );
    tags
}

#[cfg(feature = "metrics")]
pub(crate) fn get_tags_for_metric(
    resource: &Resource,
    scope: &InstrumentationLibrary,
    attrs: &AttributeSet,
) -> Tags {
    get_tags_from_attrs(
        resource.iter().chain(
            scope
                .attributes
                .iter()
                .map(|kv| (&kv.key, &kv.value))
                .chain(attrs.iter()),
        ),
    )
}

pub(crate) fn get_tags_from_attrs<'a, T>(attrs: T) -> Tags
where
    T: IntoIterator<Item = (&'a Key, &'a Value)>,
{
    let mut tags = Tags::new();

    let mut attrs_map: HashMap<_, _> = HashMap::new();
    for (k, v) in attrs.into_iter() {
        // First, allow the user to explicitly express tags with attributes that start with `ai.`
        // These attributes do not collide with any opentelemetry semantic conventions, so it is
        // assumed that the user intends for them to be a part of the `tags` portion of the
        // envelope.
        if k.as_str().starts_with("ai.") {
            if let Some(ctk) = TAG_KEY_LOOKUP.get(k) {
                tags.insert(ctk.clone(), v.to_string());
            }
        }

        attrs_map.insert(k, v);
    }

    if let Some(user_id) = attrs_map.get(&semcov::trace::ENDUSER_ID) {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id)
        // fit better?
        tags.insert(tags::USER_AUTH_USER_ID, user_id.as_str().into_owned());
    }

    if let Some(service_name) = attrs_map.get(&semcov::resource::SERVICE_NAME) {
        let mut cloud_role = service_name.as_str().into_owned();
        if let Some(service_namespace) = attrs_map.get(&semcov::resource::SERVICE_NAMESPACE) {
            cloud_role.insert(0, '.');
            cloud_role.insert_str(0, &service_namespace.as_str());
        }

        tags.insert(tags::CLOUD_ROLE, cloud_role);
    }

    if let Some(service_instance) = attrs_map.get(&semcov::resource::SERVICE_INSTANCE_ID) {
        tags.insert(
            tags::CLOUD_ROLE_INSTANCE,
            service_instance.as_str().into_owned(),
        );
    }

    if let Some(service_version) = attrs_map.get(&semcov::resource::SERVICE_VERSION) {
        tags.insert(
            tags::APPLICATION_VERSION,
            service_version.as_str().into_owned(),
        );
    }

    if let Some(sdk_name) = attrs_map.get(&semcov::resource::TELEMETRY_SDK_NAME) {
        let sdk_version = attrs_map
            .get(&semcov::resource::TELEMETRY_SDK_VERSION)
            .map(|v| v.as_str())
            .unwrap_or_else(|| "0.0.0".into());
        tags.insert(
            tags::INTERNAL_SDK_VERSION,
            format!("{}:{}", sdk_name.as_str(), sdk_version),
        );
    }

    tags
}
