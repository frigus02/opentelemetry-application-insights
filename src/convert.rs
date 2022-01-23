use crate::models::Properties;
use chrono::{DateTime, SecondsFormat, Utc};
use opentelemetry::sdk::{trace::EvictedHashMap, Resource};
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
    attributes: &EvictedHashMap,
    resource: Option<&Resource>,
) -> Option<Properties> {
    let properties_from_attrs = attributes
        .iter()
        .map(|(k, v)| (k.as_str().into(), v.into()));

    let properties = if let Some(resource) = resource {
        properties_from_attrs
            .chain(resource.iter().map(|(k, v)| (k.as_str().into(), v.into())))
            .collect()
    } else {
        properties_from_attrs.collect()
    };

    Some(properties).filter(|x: &Properties| !x.is_empty())
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
