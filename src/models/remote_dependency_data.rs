use serde::Serialize;

/// An instance of Remote Dependency represents an interaction of the monitored component with a remote component/service like SQL or an HTTP endpoint.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteDependencyData {
    pub(crate) ver: i32,
    pub(crate) name: String,
    pub(crate) id: Option<String>,
    pub(crate) result_code: Option<String>,
    pub(crate) duration: String,
    pub(crate) success: Option<bool>,
    pub(crate) data: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) type_: Option<String>,
    pub(crate) properties: Option<std::collections::BTreeMap<String, String>>,
    pub(crate) measurements: Option<std::collections::BTreeMap<String, f64>>,
}

impl Default for RemoteDependencyData {
    fn default() -> Self {
        Self {
            ver: 2,
            name: String::default(),
            id: Option::default(),
            result_code: Option::default(),
            duration: String::default(),
            success: Some(true),
            data: Option::default(),
            target: Option::default(),
            type_: Option::default(),
            properties: Option::default(),
            measurements: Option::default(),
        }
    }
}
