use crate::models::Sanitize;
use serde::Serialize;

/// Exception details of the exception in a chain.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExceptionDetails {
    /// Exception type name.
    pub(crate) type_name: String,

    /// Exception message.
    pub(crate) message: String,

    /// Text describing the stack. Either stack or parsedStack should have a value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stack: Option<String>,
}

impl Sanitize for ExceptionDetails {
    fn sanitize(&mut self) {
        self.type_name.truncate(1024);
        self.message.truncate(32768);
        if let Some(stack) = self.stack.as_mut() {
            stack.truncate(32768);
        }
    }
}
