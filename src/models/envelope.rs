use crate::models::Data;
use serde::Serialize;

/// System variables for a telemetry item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Envelope {
    pub(crate) ver: Option<i32>,
    pub(crate) name: String,
    pub(crate) time: String,
    pub(crate) sample_rate: Option<f64>,
    pub(crate) seq: Option<String>,
    pub(crate) i_key: Option<String>,
    pub(crate) flags: Option<i64>,
    pub(crate) tags: Option<std::collections::BTreeMap<String, String>>,
    pub(crate) data: Option<Data>,
}

impl Default for Envelope {
    fn default() -> Self {
        Self {
            ver: Some(1),
            name: String::default(),
            time: String::default(),
            sample_rate: Some(100.0),
            seq: Option::default(),
            i_key: Option::default(),
            flags: Option::default(),
            tags: Option::default(),
            data: Option::default(),
        }
    }
}
