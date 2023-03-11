use crate::models::LimitedLenString;
use serde::Serialize;

/// Metric data single measurement.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataPoint {
    /// Namespace of the metric.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ns: Option<LimitedLenString<256>>,

    /// Name of the metric.
    pub(crate) name: LimitedLenString<1024>,

    /// Metric type. Single measurement or the aggregated value.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<DataPointType>,

    /// Single value for measurement. Sum of individual measurements for the aggregation.
    pub(crate) value: f64,
}

/// Type of the metric data measurement.
#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
pub(crate) enum DataPointType {
    Measurement,
    Aggregation {
        /// Metric weight of the aggregated metric. Should not be set for a measurement.
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<i32>,

        /// Minimum value of the aggregated metric. Should not be set for a measurement.
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,

        /// Maximum value of the aggregated metric. Should not be set for a measurement.
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,

        /// Standard deviation of the aggregated metric. Should not be set for a measurement.
        #[serde(skip_serializing_if = "Option::is_none")]
        std_dev: Option<f64>,
    },
}
