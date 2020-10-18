use crate::models::{LimitedLenString1024, LimitedLenString32768};
use serde::Serialize;

/// Exception details of the exception in a chain.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExceptionDetails {
    /// Exception type name.
    pub(crate) type_name: LimitedLenString1024,

    /// Exception message.
    pub(crate) message: LimitedLenString32768,

    /// Text describing the stack. Either stack or parsedStack should have a value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stack: Option<LimitedLenString32768>,
}
