use crate::models::{MessageData, RemoteDependencyData, RequestData};
use serde::Serialize;

/// Data struct to contain both B and C sections.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub(crate) enum Data {
    #[serde(rename = "MessageData")]
    Message(MessageData),
    #[serde(rename = "RemoteDependencyData")]
    RemoteDependency(RemoteDependencyData),
    #[serde(rename = "RequestData")]
    Request(RequestData),
}
