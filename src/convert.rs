#[cfg(any(feature = "trace", feature = "logs"))]
use crate::models::Properties;
#[cfg(feature = "trace")]
use crate::models::{serialize_ms_links, SeverityLevel, MS_LINKS_KEY};
use chrono::{DateTime, SecondsFormat, Utc};
#[cfg(feature = "trace")]
use opentelemetry::trace::{Link, Status};
#[cfg(any(feature = "trace", feature = "logs"))]
use opentelemetry::KeyValue;
use opentelemetry::Value;
#[cfg(feature = "logs")]
use opentelemetry::{logs::AnyValue, Key};
#[cfg(any(feature = "trace", feature = "logs"))]
use opentelemetry_sdk::Resource;
#[cfg(any(feature = "trace", feature = "logs"))]
use std::collections::HashMap;
#[cfg(feature = "trace")]
use std::time::Duration;
use std::{borrow::Cow, time::SystemTime};

#[cfg(feature = "trace")]
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

#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) fn attrs_to_properties<'a, A, T: 'a>(
    attributes: A,
    resource: Option<&Resource>,
    #[cfg(feature = "trace")] links: &[Link],
) -> Option<Properties>
where
    A: Iterator<Item = &'a T> + 'a,
    &'a T: Into<AttrKeyValue<'a>>,
{
    #[allow(unused_mut)]
    let mut properties: Properties = attributes
        .map(|kv| kv.into())
        .map(|kv| (kv.0, kv.1))
        .chain(
            resource
                .iter()
                .flat_map(|r| r.iter().map(|(k, v)| (k.as_str(), v as &dyn AttrValue))),
        )
        .filter(|(k, _)| !k.starts_with("_MS."))
        .map(|(k, v)| (k.into(), v.as_str().into()))
        .collect();

    #[cfg(feature = "trace")]
    if !links.is_empty() {
        properties.insert(MS_LINKS_KEY.into(), serialize_ms_links(links).into());
    }

    Some(properties).filter(|x| !x.is_empty())
}

#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) fn attrs_to_map<'a, A, T: 'a>(attributes: A) -> HashMap<&'a str, &'a dyn AttrValue>
where
    A: Iterator<Item = &'a T> + 'a,
    &'a T: Into<AttrKeyValue<'a>>,
{
    attributes
        .map(|kv| kv.into())
        .map(|kv| (kv.0, kv.1))
        .collect()
}

#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) fn attrs_map_to_properties(
    attributes: HashMap<&str, &dyn AttrValue>,
) -> Option<Properties> {
    let properties: Properties = attributes
        .iter()
        .filter(|(&k, _)| !k.starts_with("_MS."))
        .map(|(&k, &v)| (k.into(), v.as_str().into()))
        .collect();

    Some(properties).filter(|x| !x.is_empty())
}

#[cfg(feature = "trace")]
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

#[cfg(feature = "trace")]
pub(crate) fn value_to_severity_level(value: &dyn AttrValue) -> Option<SeverityLevel> {
    match value.as_str().as_ref() {
        // Convert from `tracing` Level.
        // https://docs.rs/tracing-core/0.1.30/src/tracing_core/metadata.rs.html#526-533
        "TRACE" => Some(SeverityLevel::Verbose),
        "DEBUG" => Some(SeverityLevel::Verbose),
        "INFO" => Some(SeverityLevel::Information),
        "WARN" => Some(SeverityLevel::Warning),
        "ERROR" => Some(SeverityLevel::Error),
        _ => None,
    }
}

#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) struct AttrKeyValue<'a>(&'a str, &'a dyn AttrValue);

#[cfg(any(feature = "trace", feature = "logs"))]
impl<'a> From<&'a KeyValue> for AttrKeyValue<'a> {
    fn from(kv: &'a KeyValue) -> Self {
        AttrKeyValue(kv.key.as_str(), &kv.value as &dyn AttrValue)
    }
}

#[cfg(feature = "logs")]
impl<'a> From<&'a (Key, AnyValue)> for AttrKeyValue<'a> {
    fn from(kv: &'a (Key, AnyValue)) -> Self {
        AttrKeyValue(kv.0.as_str(), &kv.1)
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
                for &b in bytes.as_ref() {
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
                for v in list.as_ref() {
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
                for (k, v) in map.as_ref() {
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
        let attrs = [KeyValue::new("a", "b"), KeyValue::new("_MS.a", "b")];
        let resource = Resource::new([KeyValue::new("c", "d"), KeyValue::new("_MS.c", "d")]);
        let props = attrs_to_properties(attrs.iter(), Some(&resource), &[]).unwrap();
        assert_eq!(props.len(), 2);
        assert_eq!(props.get(&"a".into()).unwrap().as_ref(), "b");
        assert_eq!(props.get(&"c".into()).unwrap().as_ref(), "d");
    }

    #[test]
    fn attrs_to_properties_encodes_links() {
        let attrs: Vec<KeyValue> = Vec::new();
        let links = vec![Link::new(SpanContext::empty_context(), Vec::new(), 0)];
        let props = attrs_to_properties(attrs.iter(), None, &links).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(
            props.get(&"_MS.links".into()).unwrap().as_ref(),
            "[{\"operation_Id\":\"00000000000000000000000000000000\",\"id\":\"0000000000000000\"}]"
        );
    }

    #[test]
    fn attrs_to_properties_encodes_many_links() {
        let attrs: Vec<KeyValue> = Vec::new();
        let input_len = MS_LINKS_MAX_LEN + 10;
        let mut links = Vec::with_capacity(input_len);
        for _ in 0..input_len {
            links.push(Link::new(SpanContext::empty_context(), Vec::new(), 0));
        }
        let props = attrs_to_properties(attrs.iter(), None, &links).unwrap();
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
        let attrs = [KeyValue::new("a", "b"), KeyValue::new("_MS.a", "b")];
        let attrs_map = attrs_to_map(attrs.iter());
        assert_eq!(attrs_map.len(), 2);
        let props = attrs_map_to_properties(attrs_map).unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props.get(&"a".into()), Some(&"b".into()));
    }

    #[test_case(AnyValue::Int(1), "1" ; "int")]
    #[test_case(AnyValue::Double(1.2), "1.2" ; "double")]
    #[test_case(AnyValue::String("test".into()), "test" ; "string")]
    #[test_case(AnyValue::Boolean(true), "true" ; "boolean")]
    #[test_case(AnyValue::Bytes(Box::default()), "[]" ; "empty bytes")]
    #[test_case(AnyValue::Bytes(Box::new(vec![1, 2, 3])), "[1,2,3]" ; "bytes")]
    #[test_case(AnyValue::ListAny(Box::default()), "[]" ; "empty list")]
    #[test_case(AnyValue::ListAny(Box::new(vec![1.into(), "test".into()])), "[1,test]" ; "list")]
    #[test_case(AnyValue::Map(Box::new([].into())), "{}" ; "empty map")]
    #[test_case(AnyValue::Map(Box::new([("k1".into(), "test".into())].into())), "{k1:test}" ; "map")]
    fn any_value_as_str(v: AnyValue, expected: &'static str) {
        assert_eq!(expected.to_string(), (&v as &dyn AttrValue).as_str());
    }
}
