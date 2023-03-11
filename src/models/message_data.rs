use crate::models::{LimitedLenString32768, Properties, SeverityLevel};
use serde::Serialize;

/// Instances of Message represent printf-like trace statements that are text-searched. Log4Net,
/// NLog and other text-based log file entries are translated into intances of this type. The
/// message does not have measurements.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MessageData {
    /// Schema version
    pub(crate) ver: i32,

    /// Trace message
    pub(crate) message: LimitedLenString32768,

    /// Trace severity level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) severity_level: Option<SeverityLevel>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
