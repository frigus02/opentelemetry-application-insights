#[cfg(feature = "metrics")]
use crate::models::MetricData;
#[cfg(feature = "trace")]
use crate::models::{EventData, RemoteDependencyData, RequestData};
#[cfg(any(feature = "trace", feature = "logs"))]
use crate::models::{ExceptionData, MessageData};
use serde::Serialize;

/// Data struct to contain both B and C sections.
#[derive(Debug, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub(crate) enum Data {
    #[cfg(feature = "trace")]
    #[serde(rename = "EventData")]
    Event(EventData),
    #[cfg(any(feature = "trace", feature = "logs"))]
    #[serde(rename = "ExceptionData")]
    Exception(ExceptionData),
    #[cfg(any(feature = "trace", feature = "logs"))]
    #[serde(rename = "MessageData")]
    Message(MessageData),
    #[cfg(feature = "metrics")]
    #[serde(rename = "MetricData")]
    Metric(MetricData),
    #[cfg(feature = "trace")]
    #[serde(rename = "RemoteDependencyData")]
    RemoteDependency(RemoteDependencyData),
    #[cfg(feature = "trace")]
    #[serde(rename = "RequestData")]
    Request(RequestData),
}
