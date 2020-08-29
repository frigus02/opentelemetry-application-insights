use crate::models::{ExceptionDetails, Sanitize};
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
    pub(crate) properties: Option<std::collections::BTreeMap<String, String>>,
}

impl Sanitize for ExceptionData {
    fn sanitize(&mut self) {
        if let Some(properties) = self.properties.as_mut() {
            properties.sanitize();
        }
    }
}
