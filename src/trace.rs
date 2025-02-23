use crate::{
    convert::{
        attrs_map_to_properties, attrs_to_map, attrs_to_properties, duration_to_string,
        status_to_result_code, time_to_string, value_to_severity_level,
    },
    models::{
        context_tag_keys::attrs::CUSTOM_EVENT_NAME, Data, Envelope, EventData, ExceptionData,
        ExceptionDetails, LimitedLenString, MessageData, RemoteDependencyData, RequestData,
    },
    tags::{get_tags_for_event, get_tags_for_span},
    Exporter,
};
use opentelemetry::{
    trace::{Event, SpanKind, Status},
    Value,
};
use opentelemetry_http::HttpClient;
use opentelemetry_sdk::{
    error::OTelSdkResult,
    trace::{SpanData, SpanExporter},
    Resource,
};
use opentelemetry_semantic_conventions as semcov;
use std::{borrow::Cow, collections::HashMap, future::Future, pin::Pin, sync::Arc, time::Duration};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Deprecated semantic convention key for HTTP host
///
/// Removed in https://github.com/open-telemetry/opentelemetry-specification/pull/2469.
const DEPRECATED_HTTP_HOST: &str = "http.host";

/// Semantic convention key for HTTP 'Host' request header.
const HTTP_REQUEST_HEADER_HOST: &str = "http.request.header.host";

/// Deprecated semantic convention key for peer IP.
///
/// Replaced in https://github.com/open-telemetry/opentelemetry-specification/pull/2614 with
/// `net.sock.peer.addr`.
const DEPRECATED_NET_PEER_IP: &str = "net.peer.ip";

/// Deprecated semantic convention key for HTTP client IP.
///
/// Replaced in https://github.com/open-telemetry/opentelemetry-specification/pull/3402 with
/// `client.address`.
const DEPRECATED_HTTP_CLIENT_IP: &str = "http.client_ip";

/// Deprecated semantic convention key for client socket address.
///
/// Replaced in https://github.com/open-telemetry/opentelemetry-specification/pull/3713 with
/// `network.peer.address`.
const DEPRECATED_CLIENT_SOCKET_ADDRESS: &str = "client.socket.address";

/// Deprecated semantic convention key for server socket address.
///
/// Replaced in https://github.com/open-telemetry/opentelemetry-specification/pull/3713 with
/// `network.local.address`.
const DEPRECATED_SERVER_SOCKET_ADDRESS: &str = "server.socket.address";

/// Deprecated semantic convention key for server socket port.
///
/// Replaced in https://github.com/open-telemetry/opentelemetry-specification/pull/3713 with
/// `network.local.port`.
const DEPRECATED_SERVER_SOCKET_PORT: &str = "server.socket.port";

pub(crate) const EVENT_NAME_CUSTOM: &str = "ai.custom";
pub(crate) const EVENT_NAME_EXCEPTION: &str = "exception";

impl<C> Exporter<C> {
    fn create_envelopes_for_span(&self, span: SpanData, resource: &Resource) -> Vec<Envelope> {
        let mut result = Vec::with_capacity(1 + span.events.len());

        let (data, tags, name) = match span.span_kind {
            SpanKind::Server | SpanKind::Consumer => {
                let data: RequestData = SpanAndResource(&span, resource).into();
                let tags = get_tags_for_span(&span, resource);
                (
                    Data::Request(data),
                    tags,
                    "Microsoft.ApplicationInsights.Request",
                )
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = SpanAndResource(&span, resource).into();
                let tags = get_tags_for_span(&span, resource);
                (
                    Data::RemoteDependency(data),
                    tags,
                    "Microsoft.ApplicationInsights.RemoteDependency",
                )
            }
        };
        result.push(Envelope {
            name,
            time: time_to_string(span.start_time).into(),
            sample_rate: Some(self.sample_rate),
            i_key: Some(self.instrumentation_key.clone().into()),
            tags: Some(tags),
            data: Some(data),
        });

        let event_resource = if self.resource_attributes_in_events {
            Some(resource)
        } else {
            None
        };
        for event in span.events.iter() {
            let (data, name) = match event.name.as_ref() {
                x if x == EVENT_NAME_CUSTOM => (
                    Data::Event(EventAndResource(event, event_resource).into()),
                    "Microsoft.ApplicationInsights.Event",
                ),
                x if x == EVENT_NAME_EXCEPTION => (
                    Data::Exception(EventAndResource(event, event_resource).into()),
                    "Microsoft.ApplicationInsights.Exception",
                ),
                _ => (
                    Data::Message(EventAndResource(event, event_resource).into()),
                    "Microsoft.ApplicationInsights.Message",
                ),
            };
            result.push(Envelope {
                name,
                time: time_to_string(event.timestamp).into(),
                sample_rate: Some(self.sample_rate),
                i_key: Some(self.instrumentation_key.clone().into()),
                tags: Some(get_tags_for_event(&span, resource)),
                data: Some(data),
            });
        }

        result
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "trace")))]
impl<C> SpanExporter for Exporter<C>
where
    C: HttpClient + 'static,
{
    /// Export spans to Application Insights
    fn export(&mut self, batch: Vec<SpanData>) -> BoxFuture<'static, OTelSdkResult> {
        let client = Arc::clone(&self.client);
        let endpoint = Arc::clone(&self.endpoint);
        let envelopes: Vec<_> = batch
            .into_iter()
            .flat_map(|span| self.create_envelopes_for_span(span, &self.resource))
            .collect();

        Box::pin(async move {
            crate::uploader::send(client.as_ref(), endpoint.as_ref(), envelopes)
                .await
                .map_err(Into::into)
        })
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.resource = resource.clone();
    }
}

fn get_url_path_and_query<'v>(attrs: &HashMap<&str, &'v Value>) -> Option<Cow<'v, str>> {
    if let Some(path) = attrs.get(semcov::trace::URL_PATH) {
        if let Some(query) = attrs.get(semcov::trace::URL_QUERY) {
            Some(format!("{}?{}", path, query).into())
        } else {
            Some(path.as_str())
        }
    } else {
        attrs
            .get(
                #[allow(deprecated)]
                semcov::attribute::HTTP_TARGET,
            )
            .map(|target| target.as_str())
    }
}

fn get_server_host<'v>(attrs: &HashMap<&str, &'v Value>) -> Option<Cow<'v, str>> {
    if let Some(host) = attrs.get(HTTP_REQUEST_HEADER_HOST) {
        Some(host.as_str())
    } else if let Some(host) = attrs.get(DEPRECATED_HTTP_HOST) {
        Some(host.as_str())
    } else if let (Some(host_name), Some(host_port)) = (
        attrs.get(semcov::trace::SERVER_ADDRESS).or_else(|| {
            attrs.get(
                #[allow(deprecated)]
                semcov::attribute::NET_HOST_NAME,
            )
        }),
        attrs.get(semcov::trace::SERVER_PORT).or_else(|| {
            attrs.get(
                #[allow(deprecated)]
                semcov::attribute::NET_HOST_PORT,
            )
        }),
    ) {
        Some(format!("{}:{}", host_name.as_str(), host_port.as_str()).into())
    } else {
        None
    }
}

pub(crate) fn get_duration(span: &SpanData) -> Duration {
    span.end_time
        .duration_since(span.start_time)
        .unwrap_or_default()
}

pub(crate) fn is_request_success(span: &SpanData) -> bool {
    !matches!(span.status, Status::Error { .. })
}

pub(crate) fn is_remote_dependency_success(span: &SpanData) -> Option<bool> {
    match span.status {
        Status::Unset => None,
        Status::Ok => Some(true),
        Status::Error { .. } => Some(false),
    }
}

struct SpanAndResource<'a>(&'a SpanData, &'a Resource);

impl<'a> From<SpanAndResource<'a>> for RequestData {
    fn from(SpanAndResource(span, resource): SpanAndResource<'a>) -> RequestData {
        let mut data = RequestData {
            ver: 2,
            id: span.span_context.span_id().to_string().into(),
            name: Some(LimitedLenString::<1024>::from(span.name.clone()))
                .filter(|x| !x.as_ref().is_empty()),
            duration: duration_to_string(get_duration(span)),
            response_code: status_to_result_code(&span.status).to_string().into(),
            success: is_request_success(span),
            source: None,
            url: None,
            properties: attrs_to_properties(
                span.attributes.iter(),
                Some(resource),
                &span.links.links,
            ),
        };

        let attrs: HashMap<&str, &Value> = span
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), &kv.value))
            .collect();

        if let Some(&method) = attrs.get(semcov::trace::HTTP_REQUEST_METHOD).or_else(|| {
            #[allow(deprecated)]
            attrs.get(semcov::attribute::HTTP_METHOD)
        }) {
            data.name = Some(if let Some(route) = attrs.get(semcov::trace::HTTP_ROUTE) {
                format!("{} {}", method.as_str(), route.as_str()).into()
            } else {
                method.into()
            });
        }

        if let Some(&status_code) = attrs.get(semcov::trace::HTTP_RESPONSE_STATUS_CODE) {
            data.response_code = status_code.into();
        } else if let Some(&status_code) = attrs.get(
            #[allow(deprecated)]
            semcov::attribute::HTTP_STATUS_CODE,
        ) {
            data.response_code = status_code.into();
        }

        if let Some(&url) = attrs.get(semcov::trace::URL_FULL) {
            data.url = Some(url.into());
        } else if let Some(&url) = attrs.get(
            #[allow(deprecated)]
            semcov::attribute::HTTP_URL,
        ) {
            data.url = Some(url.into());
        } else if let Some(target) = get_url_path_and_query(&attrs) {
            let mut target = target.into_owned();
            if !target.starts_with('/') {
                target.insert(0, '/');
            }

            if let (Some(scheme), Some(host)) = (
                attrs.get(semcov::trace::URL_SCHEME).or_else(|| {
                    attrs.get(
                        #[allow(deprecated)]
                        semcov::attribute::HTTP_SCHEME,
                    )
                }),
                get_server_host(&attrs),
            ) {
                data.url = Some(format!("{}://{}{}", scheme.as_str(), host, target).into());
            } else {
                data.url = Some(target.into());
            }
        }

        if let Some(&client_address) = attrs.get(semcov::trace::CLIENT_ADDRESS) {
            data.source = Some(client_address.into());
        } else if let Some(&client_ip) = attrs.get(DEPRECATED_HTTP_CLIENT_IP) {
            data.source = Some(client_ip.into());
        } else if let Some(&peer_addr) = attrs.get(semcov::trace::NETWORK_PEER_ADDRESS) {
            data.source = Some(peer_addr.into());
        } else if let Some(&peer_addr) = attrs.get(DEPRECATED_CLIENT_SOCKET_ADDRESS) {
            data.source = Some(peer_addr.into());
        } else if let Some(&peer_addr) = attrs.get(
            #[allow(deprecated)]
            semcov::attribute::NET_SOCK_PEER_ADDR,
        ) {
            data.source = Some(peer_addr.into());
        } else if let Some(&peer_ip) = attrs.get(DEPRECATED_NET_PEER_IP) {
            data.source = Some(peer_ip.into());
        }

        data
    }
}

impl<'a> From<SpanAndResource<'a>> for RemoteDependencyData {
    fn from(SpanAndResource(span, resource): SpanAndResource<'a>) -> RemoteDependencyData {
        let mut data = RemoteDependencyData {
            ver: 2,
            id: Some(span.span_context.span_id().to_string().into()),
            name: span.name.clone().into(),
            duration: duration_to_string(get_duration(span)),
            result_code: Some(status_to_result_code(&span.status).to_string().into()),
            success: is_remote_dependency_success(span),
            data: None,
            target: None,
            type_: None,
            properties: attrs_to_properties(
                span.attributes.iter(),
                Some(resource),
                &span.links.links,
            ),
        };

        let attrs: HashMap<&str, &Value> = span
            .attributes
            .iter()
            .map(|kv| (kv.key.as_str(), &kv.value))
            .collect();

        if let Some(&status_code) = attrs.get(semcov::trace::HTTP_RESPONSE_STATUS_CODE) {
            data.result_code = Some(status_code.into());
        } else if let Some(&status_code) = attrs.get(
            #[allow(deprecated)]
            semcov::attribute::HTTP_STATUS_CODE,
        ) {
            data.result_code = Some(status_code.into());
        }

        if let Some(&url) = attrs.get(semcov::trace::URL_FULL) {
            data.data = Some(url.into());
        } else if let Some(&url) = attrs.get(
            #[allow(deprecated)]
            semcov::attribute::HTTP_URL,
        ) {
            data.data = Some(url.into());
        } else if let Some(&statement) = attrs.get(semcov::attribute::DB_QUERY_TEXT).or_else(|| {
            attrs.get(
                #[allow(deprecated)]
                semcov::attribute::DB_STATEMENT,
            )
        }) {
            data.data = Some(statement.into());
        }

        if let Some(&host) = attrs.get(HTTP_REQUEST_HEADER_HOST) {
            data.target = Some(host.into());
        } else if let Some(&host) = attrs.get(DEPRECATED_HTTP_HOST) {
            data.target = Some(host.into());
        } else if let Some(&peer_name) = attrs
            .get(semcov::trace::SERVER_ADDRESS)
            .or_else(|| attrs.get(semcov::trace::NETWORK_PEER_ADDRESS))
            .or_else(|| attrs.get(DEPRECATED_SERVER_SOCKET_ADDRESS))
            .or_else(|| {
                attrs.get(
                    #[allow(deprecated)]
                    semcov::attribute::NET_SOCK_PEER_NAME,
                )
            })
            .or_else(|| {
                attrs.get(
                    #[allow(deprecated)]
                    semcov::attribute::NET_PEER_NAME,
                )
            })
            .or_else(|| {
                attrs.get(
                    #[allow(deprecated)]
                    semcov::attribute::NET_SOCK_PEER_ADDR,
                )
            })
            .or_else(|| attrs.get(DEPRECATED_NET_PEER_IP))
        {
            if let Some(peer_port) = attrs
                .get(semcov::trace::SERVER_PORT)
                .or_else(|| attrs.get(semcov::trace::NETWORK_PEER_PORT))
                .or_else(|| attrs.get(DEPRECATED_SERVER_SOCKET_PORT))
                .or_else(|| {
                    attrs.get(
                        #[allow(deprecated)]
                        semcov::attribute::NET_SOCK_PEER_PORT,
                    )
                })
                .or_else(|| {
                    attrs.get(
                        #[allow(deprecated)]
                        semcov::attribute::NET_PEER_PORT,
                    )
                })
            {
                data.target = Some(format!("{}:{}", peer_name.as_str(), peer_port.as_str()).into());
            } else {
                data.target = Some(peer_name.into());
            }
        } else if let Some(&db_name) = attrs.get(semcov::attribute::DB_NAMESPACE).or_else(|| {
            attrs.get(
                #[allow(deprecated)]
                semcov::attribute::DB_NAME,
            )
        }) {
            data.target = Some(db_name.into());
        }

        if span.span_kind == SpanKind::Internal {
            data.type_ = Some("InProc".into());
        } else if let Some(&db_system) = attrs.get(semcov::trace::DB_SYSTEM_NAME).or_else(|| {
            attrs.get(
                #[allow(deprecated)]
                semcov::attribute::DB_SYSTEM,
            )
        }) {
            data.type_ = Some(db_system.into());
        } else if let Some(&messaging_system) = attrs.get(semcov::trace::MESSAGING_SYSTEM) {
            data.type_ = Some(messaging_system.into());
        } else if let Some(&rpc_system) = attrs.get(semcov::trace::RPC_SYSTEM) {
            data.type_ = Some(rpc_system.into());
        } else if let Some(ref properties) = data.properties {
            if properties.keys().any(|x| x.as_ref().starts_with("http.")) {
                data.type_ = Some("HTTP".into());
            } else if properties.keys().any(|x| x.as_ref().starts_with("db.")) {
                data.type_ = Some("DB".into());
            }
        }

        data
    }
}

struct EventAndResource<'a>(&'a Event, Option<&'a Resource>);

impl From<EventAndResource<'_>> for ExceptionData {
    fn from(EventAndResource(event, resource): EventAndResource<'_>) -> Self {
        let mut attrs = attrs_to_map(event.attributes.iter());
        let exception = ExceptionDetails {
            type_name: attrs
                .remove(semcov::trace::EXCEPTION_TYPE)
                .map(Into::into)
                .unwrap_or_else(|| "<no type>".into()),
            message: attrs
                .remove(semcov::trace::EXCEPTION_MESSAGE)
                .map(Into::into)
                .unwrap_or_else(|| "<no message>".into()),
            stack: attrs
                .remove(semcov::trace::EXCEPTION_STACKTRACE)
                .map(Into::into),
        };
        ExceptionData {
            ver: 2,
            exceptions: vec![exception],
            severity_level: None,
            properties: attrs_map_to_properties(attrs, resource),
        }
    }
}

impl From<EventAndResource<'_>> for EventData {
    fn from(EventAndResource(event, resource): EventAndResource<'_>) -> Self {
        let mut attrs = attrs_to_map(event.attributes.iter());
        EventData {
            ver: 2,
            name: attrs
                .remove(CUSTOM_EVENT_NAME)
                .map(Into::into)
                .unwrap_or_else(|| "<no name>".into()),
            properties: attrs_map_to_properties(attrs, resource),
        }
    }
}

/// The `tracing` create includes the severity level in an attribute called "level".
///
/// https://github.com/tokio-rs/tracing/blob/a0126b2e2d465e8e6d514acdf128fcef5b863d27/tracing-opentelemetry/src/subscriber.rs#L839
const LEVEL: &str = "level";

impl From<EventAndResource<'_>> for MessageData {
    fn from(EventAndResource(event, resource): EventAndResource<'_>) -> Self {
        let mut attrs = attrs_to_map(event.attributes.iter());
        let severity_level = attrs.get(LEVEL).and_then(|&x| value_to_severity_level(x));
        if severity_level.is_some() {
            attrs.remove(LEVEL);
        }
        MessageData {
            ver: 2,
            severity_level,
            message: if event.name.is_empty() {
                "<no message>".into()
            } else {
                event.name.clone().into_owned().into()
            },
            properties: attrs_map_to_properties(attrs, resource),
        }
    }
}
