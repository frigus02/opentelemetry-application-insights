use serde::Serialize;

/// An instance of Request represents completion of an external request to the application to do work and contains a summary of that request execution and the results.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestData {
    pub(crate) ver: i32,
    pub(crate) id: String,
    pub(crate) source: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) duration: String,
    pub(crate) response_code: String,
    pub(crate) success: bool,
    pub(crate) url: Option<String>,
    pub(crate) properties: Option<std::collections::BTreeMap<String, String>>,
    pub(crate) measurements: Option<std::collections::BTreeMap<String, f64>>,
}

impl Default for RequestData {
    fn default() -> Self {
        Self {
            ver: 2,
            id: String::default(),
            source: Option::default(),
            name: Option::default(),
            duration: String::default(),
            response_code: String::default(),
            success: true,
            url: Option::default(),
            properties: Option::default(),
            measurements: Option::default(),
        }
    }
}
