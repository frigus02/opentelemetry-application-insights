mod data;
mod envelope;
mod message_data;
mod remote_dependency_data;
mod request_data;
mod sanitize;

pub(crate) use data::*;
pub(crate) use envelope::*;
pub(crate) use message_data::*;
pub(crate) use remote_dependency_data::*;
pub(crate) use request_data::*;
pub(crate) use sanitize::*;

#[cfg(test)]
mod tests {
    use super::*;

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
}
