use crate::models::{LimitedLenString1024, LimitedLenString128, LimitedLenString8192, Properties};
use serde::Serialize;

/// An instance of Remote Dependency represents an interaction of the monitored component with a
/// remote component/service like SQL or an HTTP endpoint.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteDependencyData {
    /// Schema version
    pub(crate) ver: i32,

    /// Name of the command initiated with this dependency call. Low cardinality value. Examples
    /// are stored procedure name and URL path template.
    pub(crate) name: LimitedLenString1024,

    /// Identifier of a dependency call instance. Used for correlation with the request telemetry
    /// item corresponding to this dependency call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<LimitedLenString128>,

    /// Result code of a dependency call. Examples are SQL error code and HTTP status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result_code: Option<LimitedLenString1024>,

    /// Request duration in format: DD.HH:MM:SS.MMMMMM. Must be less than 1000 days.
    pub(crate) duration: String,

    /// Indication of successfull or unsuccessfull call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) success: Option<bool>,

    /// Command initiated by this dependency call. Examples are SQL statement and HTTP URL's with
    /// all query parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<LimitedLenString8192>,

    /// Target site of a dependency call. Examples are server name, host address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<LimitedLenString1024>,

    /// Dependency type name. Very low cardinality value for logical grouping of dependencies and
    /// interpretation of other fields like commandName and resultCode. Examples are SQL, Azure
    /// table, and HTTP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) type_: Option<LimitedLenString1024>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
