use crate::models::{LimitedLenString1024, LimitedLenString128, LimitedLenString2048, Properties};
use serde::Serialize;

/// An instance of Request represents completion of an external request to the application to do
/// work and contains a summary of that request execution and the results.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RequestData {
    /// Schema version
    pub(crate) ver: i32,

    /// Identifier of a request call instance. Used for correlation between request and other
    /// telemetry items.
    pub(crate) id: LimitedLenString128,

    /// Source of the request. Examples are the instrumentation key of the caller or the ip address
    /// of the caller.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<LimitedLenString1024>,

    /// Name of the request. Represents code path taken to process request. Low cardinality value
    /// to allow better grouping of requests. For HTTP requests it represents the HTTP method and
    /// URL path template like 'GET /values/{id}'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<LimitedLenString1024>,

    /// Request duration in format: DD.HH:MM:SS.MMMMMM. Must be less than 1000 days.
    pub(crate) duration: String,

    /// Result of a request execution. HTTP status code for HTTP requests.
    pub(crate) response_code: LimitedLenString1024,

    /// Indication of successfull or unsuccessfull call.
    pub(crate) success: bool,

    /// Request URL with all query string parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<LimitedLenString2048>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
