use crate::models::{LimitedLenString512, Properties};
use serde::Serialize;

/// Instances of Event represent structured event records that can be grouped and searched by their
/// properties. Event data item also creates a metric of event count by name.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventData {
    /// Schema version
    pub(crate) ver: i32,

    /// Event name. Keep it low cardinality to allow proper grouping and useful metrics.
    pub(crate) name: LimitedLenString512,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
