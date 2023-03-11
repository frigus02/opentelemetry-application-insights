use crate::models::LimitedLenString;
use serde::Serialize;

/// Exception details of the exception in a chain.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExceptionDetails {
    /// Exception type name.
    pub(crate) type_name: LimitedLenString<1024>,

    /// Exception message.
    pub(crate) message: LimitedLenString<32768>,

    /// Text describing the stack. Either stack or parsedStack should have a value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stack: Option<LimitedLenString<32768>>,
}
