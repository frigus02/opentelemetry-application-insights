pub(crate) trait Sanitize {
    fn sanitize(&mut self);
}

pub(crate) fn sanitize_properties(
    properties: &mut Option<std::collections::BTreeMap<String, String>>,
) {
    if let Some(properties) = properties.as_mut() {
        let keys: Vec<_> = properties
            .keys()
            .filter(|k| k.len() > 150)
            .map(|k| k.to_owned())
            .collect();
        for mut key in keys {
            let value = properties
                .remove(&key)
                .expect("value needs to exist. got key by iterating over map");
            key.truncate(150);
            properties.insert(key, value);
        }
        for value in properties.values_mut() {
            value.truncate(8192);
        }
    }
}
