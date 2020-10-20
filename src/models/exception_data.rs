use crate::models::{ExceptionDetails, Properties};
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

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
