use crate::{
    convert::AttrValue,
    models::context_tag_keys::{self as tags, Tags, TAG_KEY_LOOKUP},
};
#[cfg(feature = "trace")]
use opentelemetry::trace::{SpanId, SpanKind};
#[cfg(feature = "metrics")]
use opentelemetry::KeyValue;
use opentelemetry::{InstrumentationLibrary, Key};
#[cfg(feature = "logs")]
use opentelemetry_sdk::export::logs::LogData;
#[cfg(feature = "trace")]
use opentelemetry_sdk::export::trace::SpanData;
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions as semcov;
use std::collections::HashMap;

#[cfg(feature = "trace")]
pub(crate) fn get_tags_for_span(span: &SpanData, resource: &Resource) -> Tags {
    let mut tags = Tags::new();
    build_tags_from_resource_attrs(&mut tags, resource, &span.instrumentation_lib);

    let attrs_map = build_tags_from_attrs(
        &mut tags,
        span.attributes
            .iter()
            .map(|kv| (&kv.key, &kv.value as &dyn AttrValue)),
    );

    // Set the operation id and operation parent id.
    tags.insert(tags::OPERATION_ID, span.span_context.trace_id().to_string());
    if span.parent_span_id != SpanId::INVALID {
        tags.insert(tags::OPERATION_PARENT_ID, span.parent_span_id.to_string());
    }

    if let Some(user_id) = attrs_map.get(semcov::trace::ENDUSER_ID) {
        // Using authenticated user id here to be safe. Or would ai.user.id (anonymous user id)
        // fit better?
        tags.insert(tags::USER_AUTH_USER_ID, user_id.as_str().into_owned());
    }

    // Ensure the name of the operation is `METHOD /the/route/path`.
    if span.span_kind == SpanKind::Server || span.span_kind == SpanKind::Consumer {
        let method = attrs_map
            .get(semcov::trace::HTTP_REQUEST_METHOD)
            .or_else(|| {
                #[allow(deprecated)]
                attrs_map.get(semcov::attribute::HTTP_METHOD)
            });
        let route = attrs_map.get(semcov::trace::HTTP_ROUTE);
        if let (Some(method), Some(route)) = (method, route) {
            tags.insert(
                tags::OPERATION_NAME,
                format!("{} {}", method.as_str(), route.as_str()),
            );
        }
    }

    tags
}

#[cfg(feature = "trace")]
pub(crate) fn get_tags_for_event(span: &SpanData, resource: &Resource) -> Tags {
    let mut tags = Tags::new();
    build_tags_from_resource_attrs(&mut tags, resource, &span.instrumentation_lib);

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
    attrs: &Vec<KeyValue>,
) -> Tags {
    let mut tags = Tags::new();
    build_tags_from_resource_attrs(&mut tags, resource, scope);
    build_tags_from_attrs(
        &mut tags,
        attrs
            .iter()
            .map(|kv| (&kv.key, &kv.value as &dyn AttrValue)),
    );
    tags
}

#[cfg(feature = "logs")]
pub(crate) fn get_tags_for_log(log: &LogData, resource: &Resource) -> Tags {
    let mut tags = Tags::new();
    build_tags_from_resource_attrs(&mut tags, resource, &log.instrumentation);

    if let Some(attrs) = &log.record.attributes {
        build_tags_from_attrs(
            &mut tags,
            attrs.iter().map(|(k, v)| (k, v as &dyn AttrValue)),
        );
    }

    if let Some(trace_context) = &log.record.trace_context {
        tags.insert(tags::OPERATION_ID, trace_context.trace_id.to_string());
        tags.insert(tags::OPERATION_PARENT_ID, trace_context.span_id.to_string());
    }

    tags
}

#[cfg(feature = "live-metrics")]
pub(crate) fn get_tags_for_resource(resource: &Resource) -> Tags {
    let mut tags = Tags::new();
    build_tags_from_resource_attrs(&mut tags, resource, &Default::default());
    tags
}

fn build_tags_from_attrs<'a, T>(tags: &mut Tags, attrs: T) -> HashMap<&'a str, &'a dyn AttrValue>
where
    T: IntoIterator<Item = (&'a Key, &'a dyn AttrValue)>,
{
    let mut attrs_map: HashMap<_, _> = HashMap::new();
    for (k, v) in attrs.into_iter() {
        // First, allow the user to explicitly express tags with attributes that start with `ai.`
        // These attributes do not collide with any opentelemetry semantic conventions, so it is
        // assumed that the user intends for them to be a part of the `tags` portion of the
        // envelope.
        let k = k.as_str();
        if k.starts_with("ai.") {
            if let Some(ctk) = TAG_KEY_LOOKUP.get(k) {
                tags.insert(ctk.clone(), v.as_str().into_owned());
            }
        }

        attrs_map.insert(k, v);
    }

    attrs_map
}

fn build_tags_from_resource_attrs(
    tags: &mut Tags,
    resource: &Resource,
    instrumentation_lib: &InstrumentationLibrary,
) {
    let attrs = resource
        .iter()
        .map(|(k, v)| (k, v as &dyn AttrValue))
        .chain(
            instrumentation_lib
                .attributes
                .iter()
                .map(|kv| (&kv.key, &kv.value as &dyn AttrValue)),
        );
    let attrs_map = build_tags_from_attrs(tags, attrs);

    if let Some(service_name) = attrs_map.get(semcov::resource::SERVICE_NAME) {
        let mut cloud_role = service_name.as_str().into_owned();
        if let Some(service_namespace) = attrs_map.get(semcov::resource::SERVICE_NAMESPACE) {
            cloud_role.insert(0, '.');
            cloud_role.insert_str(0, &service_namespace.as_str());
        }

        if service_name.as_str().starts_with("unknown_service:") {
            if let Some(k8s_name) = attrs_map
                .get(semcov::resource::K8S_DEPLOYMENT_NAME)
                .or_else(|| attrs_map.get(semcov::resource::K8S_REPLICASET_NAME))
                .or_else(|| attrs_map.get(semcov::resource::K8S_STATEFULSET_NAME))
                .or_else(|| attrs_map.get(semcov::resource::K8S_JOB_NAME))
                .or_else(|| attrs_map.get(semcov::resource::K8S_CRONJOB_NAME))
                .or_else(|| attrs_map.get(semcov::resource::K8S_DAEMONSET_NAME))
            {
                cloud_role = k8s_name.as_str().into_owned();
            }
        }

        tags.insert(tags::CLOUD_ROLE, cloud_role);
    }

    if let Some(instance) = attrs_map
        .get(semcov::resource::K8S_POD_NAME)
        .or_else(|| attrs_map.get(semcov::resource::SERVICE_INSTANCE_ID))
    {
        tags.insert(tags::CLOUD_ROLE_INSTANCE, instance.as_str().into_owned());
    }

    if let Some(device_id) = attrs_map.get(semcov::resource::DEVICE_ID) {
        tags.insert(tags::DEVICE_ID, device_id.as_str().into_owned());
    }

    if let Some(device_model_name) = attrs_map.get(semcov::resource::DEVICE_MODEL_NAME) {
        tags.insert(tags::DEVICE_MODEL, device_model_name.as_str().into_owned());
    }

    if let Some(service_version) = attrs_map.get(semcov::resource::SERVICE_VERSION) {
        tags.insert(
            tags::APPLICATION_VERSION,
            service_version.as_str().into_owned(),
        );
    }

    if let Some(sdk_name) = attrs_map.get(semcov::resource::TELEMETRY_SDK_NAME) {
        let sdk_version = attrs_map
            .get(semcov::resource::TELEMETRY_SDK_VERSION)
            .map(|v| v.as_str())
            .unwrap_or_else(|| "0.0.0".into());
        tags.insert(
            tags::INTERNAL_SDK_VERSION,
            format!("{}:{}", sdk_name.as_str(), sdk_version),
        );
    }
}
