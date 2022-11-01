#[cfg(feature = "metrics")]
use crate::models::MetricData;
use crate::models::{EventData, ExceptionData, MessageData, RemoteDependencyData, RequestData};
use serde::Serialize;

/// Data struct to contain both B and C sections.
#[derive(Debug, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub(crate) enum Data {
    #[serde(rename = "EventData")]
    Event(EventData),
    #[serde(rename = "ExceptionData")]
    Exception(ExceptionData),
    #[serde(rename = "MessageData")]
    Message(MessageData),
    #[cfg(feature = "metrics")]
    #[serde(rename = "MetricData")]
    Metric(MetricData),
    #[serde(rename = "RemoteDependencyData")]
    RemoteDependency(RemoteDependencyData),
    #[serde(rename = "RequestData")]
    Request(RequestData),
}
