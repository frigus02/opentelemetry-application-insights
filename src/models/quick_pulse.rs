use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseMetric {
    pub(crate) name: &'static str,
    pub(crate) value: f64,
    pub(crate) weight: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuickPulseEnvelope {
    pub(crate) instance: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) role_name: Option<String>,
    // Use u16 instead of u8 because http::HeaderValue::try_from does not support u8.
    pub(crate) invariant_version: u16,
    pub(crate) machine_name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) metrics: Vec<QuickPulseMetric>,
    pub(crate) stream_id: String,
    pub(crate) timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<String>,
}
