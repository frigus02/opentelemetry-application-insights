pub(crate) mod context_tag_keys;
mod data;
mod envelope;
mod exception_data;
mod exception_details;
mod message_data;
mod remote_dependency_data;
mod request_data;
mod sanitize;

pub(crate) use data::*;
pub(crate) use envelope::*;
pub(crate) use exception_data::*;
pub(crate) use exception_details::*;
pub(crate) use message_data::*;
pub(crate) use remote_dependency_data::*;
pub(crate) use request_data::*;
pub(crate) use sanitize::*;

#[cfg(test)]
mod tests {
    use super::*;
    use context_tag_keys::{Tags, OPERATION_ID};

    #[test]
    fn serialization_format() {
        let envelope = Envelope {
            name: "Test".into(),
            time: "2020-06-21:10:40:00Z".into(),
            sample_rate: Some(100.0),
            i_key: None,
            tags: None,
            data: Some(Data::Message(MessageData {
                ver: 2,
                message: "hello world".into(),
                properties: None,
            })),
        };
        let serialized = serde_json::to_string(&envelope).unwrap();
        let expected = "{\"name\":\"Test\",\"time\":\"2020-06-21:10:40:00Z\",\"sampleRate\":100.0,\"data\":{\"baseType\":\"MessageData\",\"baseData\":{\"ver\":2,\"message\":\"hello world\"}}}";
        assert_eq!(expected, serialized);
    }

    #[test]
    fn sanitization() {
        let mut tags = Tags::new();
        tags.insert(OPERATION_ID, "1".repeat(200));
        let envelope = Envelope {
            name: "x".repeat(2000).into(),
            time: "2020-06-21:10:40:00Z".into(),
            sample_rate: Some(100.0),
            i_key: None,
            tags: Some(tags),
            data: Some(Data::Message(MessageData {
                ver: 2,
                message: "m".repeat(33000).into(),
                properties: None,
            })),
        };
        assert_eq!(1024, envelope.name.as_ref().len());
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
