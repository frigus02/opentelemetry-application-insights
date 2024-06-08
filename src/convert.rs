use crate::models::{serialize_ms_links, Properties, SeverityLevel, MS_LINKS_KEY};
use chrono::{DateTime, SecondsFormat, Utc};
#[cfg(feature = "logs")]
use opentelemetry::{logs::AnyValue, Key};
use opentelemetry::{
    trace::{Link, Status},
    KeyValue, Value,
};
use opentelemetry_sdk::Resource;
use std::{
    borrow::Cow,
    collections::HashMap,
    time::{Duration, SystemTime},
};

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
    links: &[Link],
) -> Option<Properties> {
    let mut properties: Properties = attributes
        .iter()
        .filter(|kv| !kv.key.as_str().starts_with("_MS."))
        .map(|kv| ((&kv.key).into(), (&kv.value).into()))
        .chain(resource.iter().map(|(k, v)| (k.into(), v.into())))
        .collect();

    if !links.is_empty() {
        properties.insert(MS_LINKS_KEY.into(), serialize_ms_links(links).into());
    }

    Some(properties).filter(|x| !x.is_empty())
}

pub(crate) fn attrs_to_map(attributes: &[KeyValue]) -> HashMap<&str, &Value> {
    attributes
        .iter()
        .map(|kv| (kv.key.as_str(), &kv.value))
        .collect()
}

#[cfg(feature = "logs")]
pub(crate) fn log_attrs_to_map(attributes: &[(Key, AnyValue)]) -> HashMap<&str, &dyn AttrValue> {
    attributes
        .iter()
        .map(|(k, v)| (k.as_str(), v as &dyn AttrValue))
        .collect()
}

pub(crate) fn attrs_map_to_properties(attributes: HashMap<&str, &Value>) -> Option<Properties> {
    let properties: Properties = attributes
        .iter()
        .filter(|(&k, _)| !k.starts_with("_MS."))
        .map(|(&k, &v)| (k.into(), v.into()))
        .collect();

    Some(properties).filter(|x| !x.is_empty())
}

#[cfg(feature = "logs")]
pub(crate) fn log_attrs_map_to_properties(
    attributes: HashMap<&str, &dyn AttrValue>,
) -> Option<Properties> {
    let properties: Properties = attributes
        .iter()
        .filter(|(&k, _)| !k.starts_with("_MS."))
        .map(|(&k, &v)| (k.into(), v.as_str().into()))
        .collect();

    Some(properties).filter(|x| !x.is_empty())
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

pub(crate) trait AttrValue {
    fn as_str(&self) -> Cow<'_, str>;
}

impl AttrValue for Value {
    fn as_str(&self) -> Cow<'_, str> {
        self.as_str()
    }
}

#[cfg(feature = "logs")]
impl AttrValue for AnyValue {
    fn as_str(&self) -> Cow<'_, str> {
        match self {
            AnyValue::Int(v) => format!("{}", v).into(),
            AnyValue::Double(v) => format!("{}", v).into(),
            AnyValue::String(v) => Cow::Borrowed(v.as_str()),
            AnyValue::Boolean(v) => format!("{}", v).into(),
            AnyValue::Bytes(bytes) => {
                let mut s = String::new();
                s.push('[');
                for &b in bytes {
                    s.push_str(&format!("{}", b));
                    s.push(',');
                }
                if !bytes.is_empty() {
                    s.pop(); // remove trailing comma
                }
                s.push(']');
                s.into()
            }
            AnyValue::ListAny(list) => {
                let mut s = String::new();
                s.push('[');
                for v in list {
                    s.push_str(v.as_str().as_ref());
                    s.push(',');
                }
                if !list.is_empty() {
                    s.pop(); // remove trailing comma
                }
                s.push(']');
                s.into()
            }
            AnyValue::Map(map) => {
                let mut s = String::new();
                s.push('{');
                for (k, v) in map {
                    s.push_str(k.as_str());
                    s.push(':');
                    s.push_str(v.as_str().as_ref());
                    s.push(',');
                }
                if !map.is_empty() {
                    s.pop(); // remove trailing comma
                }
                s.push('}');
                s.into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MS_LINKS_MAX_LEN;
    use opentelemetry::trace::SpanContext;
    use test_case::test_case;

    #[test_case(Duration::from_micros(123456789123), "1.10:17:36.789123" ; "all")]
    fn duration(duration: Duration, expected: &'static str) {
        assert_eq!(expected.to_string(), duration_to_string(duration));
    }

    #[test]
    fn attrs_to_properties_filters_ms() {
        let attrs = vec![KeyValue::new("a", "b"), KeyValue::new("_MS.a", "b")];
        let props = attrs_to_properties(&attrs, &Resource::empty(), &[]).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props.get(&"a".into()).unwrap().as_ref(), "b");
    }

    #[test]
    fn attrs_to_properties_encodes_links() {
        let links = vec![Link::new(SpanContext::empty_context(), Vec::new(), 0)];
        let props = attrs_to_properties(&[], &Resource::empty(), &links).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(
            props.get(&"_MS.links".into()).unwrap().as_ref(),
            "[{\"operation_Id\":\"00000000000000000000000000000000\",\"id\":\"0000000000000000\"}]"
        );
    }

    #[test]
    fn attrs_to_properties_encodes_many_links() {
        let input_len = MS_LINKS_MAX_LEN + 10;
        let mut links = Vec::with_capacity(input_len);
        for _ in 0..input_len {
            links.push(Link::new(SpanContext::empty_context(), Vec::new(), 0));
        }
        let props = attrs_to_properties(&[], &Resource::empty(), &links).unwrap();
        assert_eq!(props.len(), 1);
        let encoded_links = props.get(&"_MS.links".into()).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(encoded_links.as_ref()).unwrap();
        match deserialized {
            serde_json::Value::Array(arr) => assert_eq!(arr.len(), MS_LINKS_MAX_LEN),
            _ => panic!("Expected links to be serialized as JSON array"),
        }
    }

    #[test]
    fn attrs_map_to_properties_filters_ms() {
        let attrs = vec![KeyValue::new("a", "b"), KeyValue::new("_MS.a", "b")];
        let attrs_map = attrs_to_map(&attrs);
        assert_eq!(attrs_map.len(), 2);
        let props = attrs_map_to_properties(attrs_map).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props.get(&"a".into()), Some(&"b".into()));
    }
}
