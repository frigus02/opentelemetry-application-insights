use crate::{
    convert::{attrs_map_to_properties, attrs_to_map, time_to_string, AttrValue},
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
use std::{sync::Arc, time::SystemTime};

impl<C> Exporter<C> {
    fn create_envelope_for_log(&self, log: LogData) -> Envelope {
        let attrs_map = if let Some(attrs) = log.record.attributes.as_ref() {
            attrs_to_map(attrs)
        } else {
            Default::default()
        };
        let (data, name) = 'convert: {
            if let Some(&exc_type) = attrs_map.get(semcov::trace::EXCEPTION_TYPE) {
                let exception = ExceptionDetails {
                    type_name: exc_type.as_str().into_owned().into(),
                    message: attrs_map
                        .get(semcov::trace::EXCEPTION_MESSAGE)
                        .map(|&v| v.as_str().into_owned())
                        .unwrap_or_else(|| "".into())
                        .into(),
                    stack: attrs_map
                        .get(semcov::trace::EXCEPTION_STACKTRACE)
                        .map(|&v| v.as_str().into_owned().into()),
                };
                let data = Data::Exception(ExceptionData {
                    ver: 2,
                    exceptions: vec![exception],
                    severity_level: log.record.severity_number.map(Into::into),
                    properties: attrs_map_to_properties(attrs_map),
                });
                break 'convert (data, "Microsoft.ApplicationInsights.Exception");
            }

            let data = Data::Message(MessageData {
                ver: 2,
                severity_level: log.record.severity_number.map(Into::into),
                message: log
                    .record
                    .body
                    .as_ref()
                    .map(|v| (v as &dyn AttrValue).as_str().into_owned())
                    .unwrap_or_else(|| "".into())
                    .into(),
                properties: attrs_map_to_properties(attrs_map),
            });
            (data, "Microsoft.ApplicationInsights.Message")
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
        let client = Arc::clone(&self.client);
        let endpoint = Arc::clone(&self.endpoint);
        let envelopes: Vec<_> = batch
            .into_iter()
            .map(|log| self.create_envelope_for_log(log))
            .collect();

        crate::uploader::send(client.as_ref(), endpoint.as_ref(), envelopes).await?;
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
