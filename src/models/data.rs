use crate::models::{MessageData, RemoteDependencyData, RequestData};
use serde::Serialize;

/// Data struct to contain both B and C sections.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub(crate) enum Data {
    MessageData(MessageData),
    RemoteDependencyData(RemoteDependencyData),
    RequestData(RequestData),
}
