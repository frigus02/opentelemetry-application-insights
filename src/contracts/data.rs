use crate::contracts::*;
use serde::Serialize;

// NOTE: This file was automatically generated.

/// Data struct to contain both B and C sections.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "baseType", content = "baseData")]
pub enum Data {
    MessageData(MessageData),
    RemoteDependencyData(RemoteDependencyData),
    RequestData(RequestData),
}
