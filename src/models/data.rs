use crate::models::{MessageData, RemoteDependencyData, RequestData, Sanitize};
use serde::Serialize;

/// Data struct to contain both B and C sections.
#[derive(Debug, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub(crate) enum Data {
    #[serde(rename = "MessageData")]
    Message(MessageData),
    #[serde(rename = "RemoteDependencyData")]
    RemoteDependency(RemoteDependencyData),
    #[serde(rename = "RequestData")]
    Request(RequestData),
}

impl Sanitize for Data {
    fn sanitize(&mut self) {
        match self {
            Data::Message(v) => v.sanitize(),
            Data::RemoteDependency(v) => v.sanitize(),
            Data::Request(v) => v.sanitize(),
        }
    }
}
