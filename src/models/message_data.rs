use serde::Serialize;

/// Instances of Message represent printf-like trace statements that are text-searched. Log4Net, NLog and other text-based log file entries are translated into intances of this type. The message does not have measurements.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageData {
    pub(crate) ver: i32,
    pub(crate) message: String,
    pub(crate) properties: Option<std::collections::BTreeMap<String, String>>,
    pub(crate) measurements: Option<std::collections::BTreeMap<String, f64>>,
}

impl Default for MessageData {
    fn default() -> Self {
        Self {
            ver: 2,
            message: String::default(),
            properties: Option::default(),
            measurements: Option::default(),
        }
    }
}
