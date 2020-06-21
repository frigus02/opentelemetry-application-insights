use chrono::{DateTime, SecondsFormat, Utc};
use opentelemetry::api::{SpanId, TraceId, Value};
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
    let micros_remaining = micros / 1_000_000;
    format!(
        "{}.{:0>2}:{:0>2}:{:0>2}.{:0>7}",
        d, h, m, s, micros_remaining
    )
}

pub(crate) fn time_to_string(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn value_to_string(value: &Value) -> String {
    match value {
        Value::Bool(v) => v.to_string(),
        Value::I64(v) => v.to_string(),
        Value::U64(v) => v.to_string(),
        Value::F64(v) => v.to_string(),
        Value::String(v) => v.to_owned(),
        Value::Bytes(v) => base64::encode(&v),
    }
}
