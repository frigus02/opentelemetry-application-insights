use crate::models::{Properties, SeverityLevel};
use chrono::{DateTime, SecondsFormat, Utc};
use opentelemetry::{trace::Status, KeyValue, Value};
use opentelemetry_sdk::Resource;
use std::time::{Duration, SystemTime};

pub(crate) fn duration_to_string(duration: Duration) -> String {
    let micros = duration.as_micros();
    let s = micros / 1_000_000 % 60;
    let m = micros / 1_000_000 / 60 % 60;
    let h = micros / 1_000_000 / 60 / 60 % 24;
    let d = micros / 1_000_000 / 60 / 60 / 24;
    let micros_remaining = micros % 1_000_000;
    format!(
        "{}.{:0>2}:{:0>2}:{:0>2}.{:0>6}",
        d, h, m, s, micros_remaining
    )
}

pub(crate) fn time_to_string(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn attrs_to_properties(
    attributes: &[KeyValue],
    resource: &Resource,
) -> Option<Properties> {
    let properties = attributes
        .iter()
        .map(|kv| ((&kv.key).into(), (&kv.value).into()))
        .chain(resource.iter().map(|(k, v)| (k.into(), v.into())))
        .collect();

    Some(properties).filter(|x: &Properties| !x.is_empty())
}

pub(crate) fn status_to_result_code(status: &Status) -> i32 {
    // Since responseCode is a required field for RequestData, we map the span status to come kind
    // of result code representation. Numbers 1-3 were chosen because in opentelemetry@0.17.0
    // converting the StatusCode enum to an integer yielded this result.
    match status {
        Status::Unset => 0,
        Status::Ok => 1,
        Status::Error { .. } => 2,
    }
}

pub(crate) fn value_to_severity_level(value: &Value) -> Option<SeverityLevel> {
    match value.as_str().as_ref() {
        // Convert from `tracing` Level.
        // https://docs.rs/tracing-core/0.1.30/src/tracing_core/metadata.rs.html#526-533
        "TRACE" => Some(SeverityLevel::Verbose),
        "DEBUG" => Some(SeverityLevel::Information),
        "INFO" => Some(SeverityLevel::Information),
        "WARN" => Some(SeverityLevel::Warning),
        "ERROR" => Some(SeverityLevel::Error),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(Duration::from_micros(123456789123), "1.10:17:36.789123" ; "all")]
    fn duration(duration: Duration, expected: &'static str) {
        assert_eq!(expected.to_string(), duration_to_string(duration));
    }
}
