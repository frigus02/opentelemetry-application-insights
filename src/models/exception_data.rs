use crate::models::{ExceptionDetails, Properties, SeverityLevel};
use serde::Serialize;

/// An instance of Exception represents a handled or unhandled exception that occurred during
/// execution of the monitored application.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExceptionData {
    /// Schema version
    pub(crate) ver: i32,

    /// Exception chain - list of inner exceptions.
    pub(crate) exceptions: Vec<ExceptionDetails>,

    /// Severity level. Mostly used to indicate exception severity level when it is reported by
    /// logging library.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) severity_level: Option<SeverityLevel>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
