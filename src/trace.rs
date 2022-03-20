use crate::StreamingHttpClient;
use crate::{
    convert::{attrs_to_properties, duration_to_string, time_to_string},
    models::{
        Data, Envelope, ExceptionData, ExceptionDetails, LimitedLenString1024, MessageData,
        Properties, RemoteDependencyData, RequestData,
    },
    tags::{get_tags_for_event, get_tags_for_span},
    Exporter,
};
use async_trait::async_trait;
use opentelemetry::{
    sdk::export::trace::{ExportResult, SpanData, SpanExporter},
    trace::{Event, SpanKind, StatusCode},
    Key, Value,
};
use opentelemetry_semantic_conventions as semcov;
use std::collections::HashMap;

impl<C> Exporter<C> {
    fn create_envelopes_for_span(&self, span: SpanData) -> Vec<Envelope> {
        let mut result = Vec::with_capacity(1 + span.events.len());

        let (data, tags, name) = match span.span_kind {
            SpanKind::Server | SpanKind::Consumer => {
                let data: RequestData = (&span).into();
                let tags = get_tags_for_span(&span);
                (
                    Data::Request(data),
                    tags,
                    "Microsoft.ApplicationInsights.Request",
                )
            }
            SpanKind::Client | SpanKind::Producer | SpanKind::Internal => {
                let data: RemoteDependencyData = (&span).into();
                let tags = get_tags_for_span(&span);
                (
                    Data::RemoteDependency(data),
                    tags,
                    "Microsoft.ApplicationInsights.RemoteDependency",
                )
            }
        };
        result.push(Envelope {
            name: name.into(),
            time: time_to_string(span.start_time).into(),
            sample_rate: Some(self.sample_rate),
            i_key: Some(self.instrumentation_key.clone().into()),
            tags: Some(tags),
            data: Some(data),
        });

        for event in span.events.iter() {
            let (data, name) = match event.name.as_ref() {
                "exception" => (
                    Data::Exception(event.into()),
                    "Microsoft.ApplicationInsights.Exception",
                ),
                _ => (
                    Data::Message(event.into()),
                    "Microsoft.ApplicationInsights.Message",
                ),
            };
            result.push(Envelope {
                name: name.into(),
                time: time_to_string(event.timestamp).into(),
                sample_rate: Some(self.sample_rate),
                i_key: Some(self.instrumentation_key.clone().into()),
                tags: Some(get_tags_for_event(&span)),
                data: Some(data),
            });
        }

        result
    }
}

#[async_trait]
impl<C> SpanExporter for Exporter<C>
where
    C: StreamingHttpClient,
{
    /// Export spans to Application Insights
    async fn export(&mut self, batch: Vec<SpanData>) -> ExportResult {
        let envelopes: Vec<_> = batch
            .into_iter()
            .flat_map(|span| self.create_envelopes_for_span(span))
            .collect();

        crate::uploader::send(&self.client, &self.endpoint, envelopes).await?;
        Ok(())
    }
}

impl From<&SpanData> for RequestData {
    fn from(span: &SpanData) -> RequestData {
        let mut data = RequestData {
            ver: 2,
            id: span.span_context.span_id().to_string().into(),
            name: Some(LimitedLenString1024::from(span.name.clone()))
                .filter(|x| !x.as_ref().is_empty()),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            response_code: (span.status_code as i32).to_string().into(),
            success: span.status_code != StatusCode::Error,
            source: None,
            url: None,
            properties: attrs_to_properties(&span.attributes, span.resource.as_deref()),
        };

        if let Some(method) = span.attributes.get(&semcov::trace::HTTP_METHOD) {
            data.name = Some(
                if let Some(route) = span.attributes.get(&semcov::trace::HTTP_ROUTE) {
                    format!("{} {}", method.as_str(), route.as_str()).into()
                } else {
                    method.into()
                },
            );
        }

        if let Some(status_code) = span.attributes.get(&semcov::trace::HTTP_STATUS_CODE) {
            data.response_code = status_code.into();
        }

        if let Some(url) = span.attributes.get(&semcov::trace::HTTP_URL) {
            data.url = Some(url.into());
        } else if let Some(target) = span.attributes.get(&semcov::trace::HTTP_TARGET) {
            let mut target = target.as_str().into_owned();
            if !target.starts_with('/') {
                target.insert(0, '/');
            }

            if let (Some(scheme), Some(host)) = (
                span.attributes.get(&semcov::trace::HTTP_SCHEME),
                span.attributes.get(&semcov::trace::HTTP_HOST),
            ) {
                data.url =
                    Some(format!("{}://{}{}", scheme.as_str(), host.as_str(), target).into());
            } else {
                data.url = Some(target.into());
            }
        }

        if let Some(client_ip) = span.attributes.get(&semcov::trace::HTTP_CLIENT_IP) {
            data.source = Some(client_ip.into());
        } else if let Some(peer_ip) = span.attributes.get(&semcov::trace::NET_PEER_IP) {
            data.source = Some(peer_ip.into());
        }

        data
    }
}

impl From<&SpanData> for RemoteDependencyData {
    fn from(span: &SpanData) -> RemoteDependencyData {
        let mut data = RemoteDependencyData {
            ver: 2,
            id: Some(span.span_context.span_id().to_string().into()),
            name: span.name.clone().into(),
            duration: duration_to_string(
                span.end_time
                    .duration_since(span.start_time)
                    .unwrap_or_default(),
            ),
            result_code: Some((span.status_code as i32).to_string().into()),
            success: match span.status_code {
                StatusCode::Unset => None,
                StatusCode::Ok => Some(true),
                StatusCode::Error => Some(false),
            },
            data: None,
            target: None,
            type_: None,
            properties: attrs_to_properties(&span.attributes, span.resource.as_deref()),
        };

        if let Some(status_code) = span.attributes.get(&semcov::trace::HTTP_STATUS_CODE) {
            data.result_code = Some(status_code.into());
        }

        if let Some(url) = span.attributes.get(&semcov::trace::HTTP_URL) {
            data.data = Some(url.into());
        } else if let Some(statement) = span.attributes.get(&semcov::trace::DB_STATEMENT) {
            data.data = Some(statement.into());
        }

        if let Some(host) = span.attributes.get(&semcov::trace::HTTP_HOST) {
            data.target = Some(host.into());
        } else if let Some(peer_name) = span.attributes.get(&semcov::trace::NET_PEER_NAME) {
            if let Some(peer_port) = span.attributes.get(&semcov::trace::NET_PEER_PORT) {
                data.target = Some(format!("{}:{}", peer_name.as_str(), peer_port.as_str()).into());
            } else {
                data.target = Some(peer_name.into());
            }
        } else if let Some(peer_ip) = span.attributes.get(&semcov::trace::NET_PEER_IP) {
            if let Some(peer_port) = span.attributes.get(&semcov::trace::NET_PEER_PORT) {
                data.target = Some(format!("{}:{}", peer_ip.as_str(), peer_port.as_str()).into());
            } else {
                data.target = Some(peer_ip.into());
            }
        } else if let Some(db_name) = span.attributes.get(&semcov::trace::DB_NAME) {
            data.target = Some(db_name.into());
        }

        if span.span_kind == SpanKind::Internal {
            data.type_ = Some("InProc".into());
        } else if let Some(db_system) = span.attributes.get(&semcov::trace::DB_SYSTEM) {
            data.type_ = Some(db_system.into());
        } else if let Some(messaging_system) = span.attributes.get(&semcov::trace::MESSAGING_SYSTEM)
        {
            data.type_ = Some(messaging_system.into());
        } else if let Some(rpc_system) = span.attributes.get(&semcov::trace::RPC_SYSTEM) {
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

impl From<&Event> for ExceptionData {
    fn from(event: &Event) -> ExceptionData {
        let mut attrs: HashMap<&Key, &Value> = event
            .attributes
            .iter()
            .map(|kv| (&kv.key, &kv.value))
            .collect();
        let exception = ExceptionDetails {
            type_name: attrs
                .remove(&semcov::trace::EXCEPTION_TYPE)
                .map(Into::into)
                .unwrap_or_else(|| "<no type>".into()),
            message: attrs
                .remove(&semcov::trace::EXCEPTION_MESSAGE)
                .map(Into::into)
                .unwrap_or_else(|| "<no message>".into()),
            stack: attrs
                .remove(&semcov::trace::EXCEPTION_STACKTRACE)
                .map(Into::into),
        };
        ExceptionData {
            ver: 2,
            exceptions: vec![exception],
            properties: Some(
                attrs
                    .iter()
                    .map(|(k, v)| (k.as_str().into(), (*v).into()))
                    .collect(),
            )
            .filter(|x: &Properties| !x.is_empty()),
        }
    }
}

impl From<&Event> for MessageData {
    fn from(event: &Event) -> MessageData {
        MessageData {
            ver: 2,
            message: if event.name.is_empty() {
                "<no message>".into()
            } else {
                event.name.clone().into_owned().into()
            },
            properties: Some(
                event
                    .attributes
                    .iter()
                    .map(|kv| (kv.key.as_str().into(), (&kv.value).into()))
                    .collect(),
            )
            .filter(|x: &Properties| !x.is_empty()),
        }
    }
}
