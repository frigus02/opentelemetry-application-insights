use crate::models::Properties;
use chrono::{DateTime, SecondsFormat, Utc};
use opentelemetry::{
    sdk::{trace::EvictedHashMap, Resource},
    trace::{SpanId, TraceId},
};
use std::time::{Duration, SystemTime};
use std::sync::Arc;

pub(crate) fn trace_id_to_string(trace_id: TraceId) -> String {
    format!("{:032x}", trace_id.to_u128())
}

pub(crate) fn span_id_to_string(span_id: SpanId) -> String {
    format!("{:016x}", span_id.to_u64())
}

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
    attributes: &EvictedHashMap,
    resource: Option<Arc<Resource>>,
) -> Option<Properties> {
    let properties = attributes
        .iter()
        .map(|(k, v)| (k.as_str().into(), v.into()));

    if let Some(resource) = resource {
        Some(
            properties
                .chain(resource.iter().map(|(k, v)| (k.as_str().into(), v.into())))
                .collect(),
        )
        .filter(|x: &Properties| !x.is_empty())
    } else {
        Some(
            properties.collect(),
        )
        .filter(|x: &Properties| !x.is_empty())
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case(TraceId::invalid(),            "00000000000000000000000000000000" ; "zero")]
    #[test_case(TraceId::from_u128(314),       "0000000000000000000000000000013a" ; "some number")]
    #[test_case(TraceId::from_u128(u128::MAX), "ffffffffffffffffffffffffffffffff" ; "max")]
    fn trace_id(id: TraceId, expected: &'static str) {
        assert_eq!(expected.to_string(), trace_id_to_string(id));
    }

    #[test_case(SpanId::invalid(),          "0000000000000000" ; "zero")]
    #[test_case(SpanId::from_u64(314),      "000000000000013a" ; "some number")]
    #[test_case(SpanId::from_u64(u64::MAX), "ffffffffffffffff" ; "max")]
    fn span_id(id: SpanId, expected: &'static str) {
        assert_eq!(expected.to_string(), span_id_to_string(id));
    }

    #[test_case(Duration::from_micros(123456789123), "1.10:17:36.789123" ; "all")]
    fn duration(duration: Duration, expected: &'static str) {
        assert_eq!(expected.to_string(), duration_to_string(duration));
    }
}
