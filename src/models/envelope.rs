use crate::models::Data;
use serde::Serialize;

/// System variables for a telemetry item.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Envelope {
    pub(crate) name: String,
    pub(crate) time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sample_rate: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) i_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tags: Option<std::collections::BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<Data>,
}
