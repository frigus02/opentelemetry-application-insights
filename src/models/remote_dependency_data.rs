use crate::models::Sanitize;
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
    pub(crate) name: String,

    /// Identifier of a dependency call instance. Used for correlation with the request telemetry
    /// item corresponding to this dependency call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<String>,

    /// Result code of a dependency call. Examples are SQL error code and HTTP status code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) result_code: Option<String>,

    /// Request duration in format: DD.HH:MM:SS.MMMMMM. Must be less than 1000 days.
    pub(crate) duration: String,

    /// Indication of successfull or unsuccessfull call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) success: Option<bool>,

    /// Command initiated by this dependency call. Examples are SQL statement and HTTP URL's with
    /// all query parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<String>,

    /// Target site of a dependency call. Examples are server name, host address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target: Option<String>,

    /// Dependency type name. Very low cardinality value for logical grouping of dependencies and
    /// interpretation of other fields like commandName and resultCode. Examples are SQL, Azure
    /// table, and HTTP.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) type_: Option<String>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<std::collections::BTreeMap<String, String>>,
}

impl Sanitize for RemoteDependencyData {
    fn sanitize(&mut self) {
        self.name.truncate(1024);
        if let Some(id) = self.id.as_mut() {
            id.truncate(128);
        }
        if let Some(result_code) = self.result_code.as_mut() {
            result_code.truncate(1024);
        }
        if let Some(data) = self.data.as_mut() {
            data.truncate(8192);
        }
        if let Some(type_) = self.type_.as_mut() {
            type_.truncate(1024);
        }
        if let Some(target) = self.target.as_mut() {
            target.truncate(1024);
        }
        if let Some(properties) = self.properties.as_mut() {
            properties.sanitize();
        }
    }
}
