use crate::models::{DataPoint, Properties};
use serde::Serialize;

/// An instance of the Metric item is a list of measurements (single data points) and/or
/// aggregations.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetricData {
    /// Schema version
    pub(crate) ver: i32,

    /// List of metrics. Only one metric in the list is currently supported by Application Insights
    /// storage. If multiple data points were sent only the first one will be used.
    pub(crate) metrics: Vec<DataPoint>,

    /// Collection of custom properties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) properties: Option<Properties>,
}
