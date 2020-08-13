use chrono::{DateTime, SecondsFormat, Utc};
use opentelemetry::api::{SpanId, TraceId, Value};
use opentelemetry::sdk::{EvictedHashMap, Resource};
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};

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
        "{}.{:0>2}:{:0>2}:{:0>2}.{:0>7}",
        d, h, m, s, micros_remaining
    )
}

pub(crate) fn time_to_string(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn collect_attrs<'a>(
    attributes: &'a EvictedHashMap,
    resource: &'a Resource,
) -> HashMap<&'a str, &'a Value> {
    attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v))
        .chain(resource.iter().map(|(k, v)| (k.as_str(), v)))
        .collect()
}

pub(crate) fn attrs_to_properties(
    mut attrs: HashMap<&str, &Value>,
) -> Option<BTreeMap<String, String>> {
    Some(
        attrs
            .drain()
            .map(|(k, v)| (k.to_string(), v.into()))
            .collect(),
    )
    .filter(|x: &BTreeMap<String, String>| !x.is_empty())
}

pub(crate) fn otel_to_semantic_version(otel: &str) -> String {
    if otel.is_empty() {
        "0.0.0".into()
    } else if otel.starts_with("semver:") {
        otel["semver:".len()..].into()
    } else {
        format!(
            "0.0.0-{}",
            if let Some(i) = otel.find(':') {
                &otel[i + 1..]
            } else {
                otel
            }
        )
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

    #[test_case(Duration::from_micros(123456789123), "1.10:17:36.0789123" ; "all")]
    fn duration(duration: Duration, expected: &'static str) {
        assert_eq!(expected.to_string(), duration_to_string(duration));
    }

    #[test_case("semver:1.2.3",     "1.2.3"                  ; "semver")]
    #[test_case("git:8ae73a",       "0.0.0-8ae73a"           ; "git sha")]
    #[test_case("0.0.4.2.20190921", "0.0.0-0.0.4.2.20190921" ; "untyped")]
    #[test_case("",                 "0.0.0"                  ; "empty")]
    fn semantic_version(otel_version: &'static str, expected: &'static str) {
        assert_eq!(expected.to_string(), otel_to_semantic_version(otel_version));
    }
}
