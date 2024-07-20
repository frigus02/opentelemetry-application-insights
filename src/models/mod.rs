pub(crate) mod context_tag_keys;
mod data;
#[cfg(feature = "metrics")]
mod data_point;
mod envelope;
#[cfg(feature = "trace")]
mod event_data;
#[cfg(any(feature = "trace", feature = "logs"))]
mod exception_data;
#[cfg(any(feature = "trace", feature = "logs"))]
mod exception_details;
#[cfg(any(feature = "trace", feature = "logs"))]
mod message_data;
#[cfg(feature = "metrics")]
mod metric_data;
#[cfg(feature = "trace")]
mod ms_link;
#[cfg(feature = "live-metrics")]
mod quick_pulse;
#[cfg(feature = "trace")]
mod remote_dependency_data;
#[cfg(feature = "trace")]
mod request_data;
mod sanitize;
#[cfg(any(feature = "trace", feature = "logs"))]
mod severity_level;

pub(crate) use data::*;
#[cfg(feature = "metrics")]
pub(crate) use data_point::*;
pub(crate) use envelope::*;
#[cfg(feature = "trace")]
pub(crate) use event_data::*;
#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) use exception_data::*;
#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) use exception_details::*;
#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) use message_data::*;
#[cfg(feature = "metrics")]
pub(crate) use metric_data::*;
#[cfg(feature = "trace")]
pub(crate) use ms_link::*;
#[cfg(feature = "live-metrics")]
pub(crate) use quick_pulse::*;
#[cfg(feature = "trace")]
pub(crate) use remote_dependency_data::*;
#[cfg(feature = "trace")]
pub(crate) use request_data::*;
pub(crate) use sanitize::*;
#[cfg(any(feature = "trace", feature = "logs"))]
pub(crate) use severity_level::*;

#[cfg(test)]
mod tests {
    use super::*;
    use context_tag_keys::{Tags, OPERATION_ID};

    #[test]
    fn serialization_format() {
        let envelope = Envelope {
            name: "Test",
            time: "2020-06-21:10:40:00Z".into(),
            sample_rate: Some(100.0),
            i_key: None,
            tags: None,
            data: Some(Data::Message(MessageData {
                ver: 2,
                severity_level: None,
                message: "hello world".into(),
                properties: None,
            })),
        };
        let serialized = serde_json::to_string(&envelope).unwrap();
        let expected = "{\"name\":\"Test\",\"time\":\"2020-06-21:10:40:00Z\",\"sampleRate\":100.0,\"data\":{\"baseType\":\"MessageData\",\"baseData\":{\"ver\":2,\"message\":\"hello world\"}}}";
        assert_eq!(expected, serialized);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn serialization_format_metrics() {
        let envelope = Envelope {
            name: "Test",
            time: "2020-06-21:10:40:00Z".into(),
            sample_rate: Some(100.0),
            i_key: None,
            tags: None,
            data: Some(Data::Metric(MetricData {
                ver: 2,
                metrics: vec![DataPoint {
                    ns: None,
                    name: "hello world".into(),
                    kind: Some(DataPointType::Measurement),
                    value: 42.0,
                }],
                properties: None,
            })),
        };
        let serialized = serde_json::to_string(&envelope).unwrap();
        let expected = "{\"name\":\"Test\",\"time\":\"2020-06-21:10:40:00Z\",\"sampleRate\":100.0,\"data\":{\"baseType\":\"MetricData\",\"baseData\":{\"ver\":2,\"metrics\":[{\"name\":\"hello world\",\"kind\":\"Measurement\",\"value\":42.0}]}}}";
        assert_eq!(expected, serialized);
    }

    #[test]
    fn sanitization() {
        let mut tags = Tags::new();
        tags.insert(OPERATION_ID, "1".repeat(200));
        let envelope = Envelope {
            name: "Test",
            time: "2020-06-21:10:40:00Z".into(),
            sample_rate: Some(100.0),
            i_key: None,
            tags: Some(tags),
            data: Some(Data::Message(MessageData {
                ver: 2,
                severity_level: None,
                message: "m".repeat(33000).into(),
                properties: None,
            })),
        };
        assert_eq!(
            128,
            envelope.tags.unwrap().get(&OPERATION_ID).unwrap().len()
        );
        assert_eq!(
            32768,
            match envelope.data.unwrap() {
                Data::Message(data) => data.message.as_ref().len(),
                _ => panic!("we should not get here"),
            }
        );
    }
}
