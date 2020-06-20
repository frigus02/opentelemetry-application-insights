use serde::Serialize;
use crate::contracts::*;

// NOTE: This file was automatically generated.

/// Type of the metric data measurement.
#[derive(Debug, Clone, Serialize)]
pub enum DataPointType {
    Measurement,
    Aggregation,
}