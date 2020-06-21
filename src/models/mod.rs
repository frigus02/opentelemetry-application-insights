mod data;
mod envelope;
mod message_data;
mod remote_dependency_data;
mod request_data;

pub(crate) use data::*;
pub(crate) use envelope::*;
pub(crate) use message_data::*;
pub(crate) use remote_dependency_data::*;
pub(crate) use request_data::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_format() {
        let envelope = Envelope {
            data: Some(Data::Message(MessageData::default())),
            ..Envelope::default()
        };
        let serialized = serde_json::to_string(&envelope).unwrap();
        let expected = "{\"name\":\"\",\"time\":\"\",\"sampleRate\":100.0,\"data\":{\"baseType\":\"MessageData\",\"baseData\":{\"ver\":2,\"message\":\"\"}}}";
        assert_eq!(expected, serialized);
    }
}
