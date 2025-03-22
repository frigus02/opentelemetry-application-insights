use crate::{
    convert::{
        attrs_map_to_properties, attrs_to_map, attrs_to_properties, time_to_string, AttrValue,
    },
    models::{Data, Envelope, ExceptionData, ExceptionDetails, MessageData, SeverityLevel},
    tags::get_tags_for_log,
    Exporter,
};
use opentelemetry::{logs::Severity, InstrumentationScope};
use opentelemetry_http::HttpClient;
use opentelemetry_sdk::{
    error::OTelSdkResult,
    logs::{LogBatch, LogExporter, SdkLogRecord},
    Resource,
};
use opentelemetry_semantic_conventions as semcov;
use std::{sync::Arc, time::SystemTime};

fn is_exception(record: &SdkLogRecord) -> bool {
    record.attributes_iter().any(|(k, _)| {
        k.as_str() == semcov::trace::EXCEPTION_TYPE
            || k.as_str() == semcov::trace::EXCEPTION_MESSAGE
    })
}

impl<C> Exporter<C> {
    fn create_envelope_for_log(
        &self,
        (record, instrumentation_scope): (&SdkLogRecord, &InstrumentationScope),
    ) -> Envelope {
        let event_resource = if self.resource_attributes_in_events_and_logs {
            Some(&self.resource)
        } else {
            None
        };
        let (data, name) = if is_exception(record) {
            (
                Data::Exception(RecordAndResource(record, event_resource).into()),
                "Microsoft.ApplicationInsights.Exception",
            )
        } else {
            (
                Data::Message(RecordAndResource(record, event_resource).into()),
                "Microsoft.ApplicationInsights.Message",
            )
        };

        Envelope {
            name,
            time: time_to_string(
                record
                    .timestamp()
                    .or(record.observed_timestamp())
                    .unwrap_or_else(SystemTime::now),
            )
            .into(),
            sample_rate: None,
            i_key: Some(self.instrumentation_key.clone().into()),
            tags: Some(get_tags_for_log(
                record,
                instrumentation_scope,
                &self.resource,
            )),
            data: Some(data),
        }
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
impl<C> LogExporter for Exporter<C>
where
    C: HttpClient + 'static,
{
    fn export(
        &self,
        batch: LogBatch<'_>,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let client = Arc::clone(&self.client);
        let endpoint = Arc::clone(&self.track_endpoint);
        let envelopes: Vec<_> = batch
            .iter()
            .map(|log| self.create_envelope_for_log(log))
            .collect();

        async move {
            crate::uploader::send(client.as_ref(), endpoint.as_ref(), envelopes)
                .await
                .map_err(Into::into)
        }
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.resource = resource.clone();
    }
}

impl From<Severity> for SeverityLevel {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Trace
            | Severity::Trace2
            | Severity::Trace3
            | Severity::Trace4
            | Severity::Debug
            | Severity::Debug2
            | Severity::Debug3
            | Severity::Debug4 => SeverityLevel::Verbose,
            Severity::Info | Severity::Info2 | Severity::Info3 | Severity::Info4 => {
                SeverityLevel::Information
            }
            Severity::Warn | Severity::Warn2 | Severity::Warn3 | Severity::Warn4 => {
                SeverityLevel::Warning
            }
            Severity::Error | Severity::Error2 | Severity::Error3 | Severity::Error4 => {
                SeverityLevel::Error
            }
            Severity::Fatal | Severity::Fatal2 | Severity::Fatal3 | Severity::Fatal4 => {
                SeverityLevel::Critical
            }
        }
    }
}

struct RecordAndResource<'a>(&'a SdkLogRecord, Option<&'a Resource>);

impl From<RecordAndResource<'_>> for ExceptionData {
    fn from(RecordAndResource(record, resource): RecordAndResource) -> ExceptionData {
        let mut attrs = attrs_to_map(record.attributes_iter());
        let exception = ExceptionDetails {
            type_name: attrs
                .remove(semcov::trace::EXCEPTION_TYPE)
                .map(Into::into)
                .unwrap_or_else(|| "".into()),
            message: attrs
                .remove(semcov::trace::EXCEPTION_MESSAGE)
                .map(Into::into)
                .unwrap_or_else(|| "".into()),
            stack: attrs
                .remove(semcov::trace::EXCEPTION_STACKTRACE)
                .map(Into::into),
        };
        ExceptionData {
            ver: 2,
            exceptions: vec![exception],
            severity_level: record.severity_number().map(Into::into),
            properties: attrs_map_to_properties(attrs, resource),
        }
    }
}

impl From<RecordAndResource<'_>> for MessageData {
    fn from(RecordAndResource(record, resource): RecordAndResource) -> MessageData {
        MessageData {
            ver: 2,
            severity_level: record.severity_number().map(Into::into),
            message: record
                .body()
                .as_ref()
                .map(|v| v.as_str().into_owned())
                .unwrap_or_else(|| "".into())
                .into(),
            properties: attrs_to_properties(
                record.attributes_iter(),
                resource,
                #[cfg(feature = "trace")]
                &[],
            ),
        }
    }
}
