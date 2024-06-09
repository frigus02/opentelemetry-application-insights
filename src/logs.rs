use crate::{
    convert::{
        attrs_map_to_properties, attrs_to_map, attrs_to_properties, time_to_string, AttrValue,
    },
    models::{Data, Envelope, ExceptionData, ExceptionDetails, MessageData, SeverityLevel},
    tags::get_tags_for_log,
    Exporter,
};
use async_trait::async_trait;
use opentelemetry::logs::{LogResult, Severity};
use opentelemetry_http::HttpClient;
use opentelemetry_sdk::{
    export::logs::{LogData, LogExporter},
    Resource,
};
use opentelemetry_semantic_conventions as semcov;
use std::time::SystemTime;

fn is_exception(log: &LogData) -> bool {
    if let Some(attrs) = &log.record.attributes {
        attrs.iter().any(|(k, _)| {
            k.as_str() == semcov::trace::EXCEPTION_TYPE
                || k.as_str() == semcov::trace::EXCEPTION_MESSAGE
        })
    } else {
        false
    }
}

impl<C> Exporter<C> {
    fn create_envelope_for_log(&self, log: LogData) -> Envelope {
        let (data, name) = if is_exception(&log) {
            (
                Data::Exception((&log).into()),
                "Microsoft.ApplicationInsights.Exception",
            )
        } else {
            (
                Data::Message((&log).into()),
                "Microsoft.ApplicationInsights.Message",
            )
        };

        Envelope {
            name,
            time: time_to_string(
                log.record
                    .timestamp
                    .or(log.record.observed_timestamp)
                    .unwrap_or_else(SystemTime::now),
            )
            .into(),
            sample_rate: Some(self.sample_rate),
            i_key: Some(self.instrumentation_key.clone().into()),
            tags: Some(get_tags_for_log(&log, &self.resource)),
            data: Some(data),
        }
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "logs")))]
#[async_trait]
impl<C> LogExporter for Exporter<C>
where
    C: HttpClient + 'static,
{
    async fn export(&mut self, batch: Vec<LogData>) -> LogResult<()> {
        let envelopes: Vec<_> = batch
            .into_iter()
            .map(|log| self.create_envelope_for_log(log))
            .collect();

        crate::uploader::send(self.client.as_ref(), &self.endpoint, envelopes).await?;
        Ok(())
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

impl From<&LogData> for ExceptionData {
    fn from(log: &LogData) -> ExceptionData {
        let mut attrs = if let Some(attrs) = log.record.attributes.as_ref() {
            attrs_to_map(attrs)
        } else {
            Default::default()
        };
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
            severity_level: log.record.severity_number.map(Into::into),
            properties: attrs_map_to_properties(attrs),
        }
    }
}

impl From<&LogData> for MessageData {
    fn from(log: &LogData) -> MessageData {
        MessageData {
            ver: 2,
            severity_level: log.record.severity_number.map(Into::into),
            message: log
                .record
                .body
                .as_ref()
                .map(|v| v.as_str().into_owned())
                .unwrap_or_else(|| "".into())
                .into(),
            properties: log
                .record
                .attributes
                .as_ref()
                .and_then(|attrs| attrs_to_properties(attrs, None, &[])),
        }
    }
}
