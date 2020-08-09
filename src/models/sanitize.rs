use log::debug;
use std::collections::BTreeMap;

pub(crate) trait Sanitize {
    fn sanitize(&mut self);
}

impl Sanitize for BTreeMap<String, String> {
    fn sanitize(&mut self) {
        let long_keys: Vec<_> = self
            .keys()
            .filter(|k| k.len() > 150)
            .map(|k| k.to_owned())
            .collect();
        for mut long_key in long_keys {
            let (mut key, value) = self
                .remove_entry(&long_key)
                .expect("value needs to exist. got key by iterating over map");
            key.truncate(150);
            if self.insert(key, value).is_some() {
                long_key.truncate(150);
                debug!(
                    "Truncated property name overrides property with the same name: {}",
                    long_key
                );
            }
        }
        for value in self.values_mut() {
            value.truncate(8192);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn sanitize_properties() {
        let mut properties = BTreeMap::from_iter(vec![
            // Long value
            ("1".repeat(1), "v".repeat(8200)),
            // Long key and long value
            ("2".repeat(160), "v".repeat(8200)),
            // Long key
            ("3".repeat(160), "v".repeat(1)),
            // Long key collides with and replaces other key
            ("4".repeat(150), "x".repeat(1)),
            ("4".repeat(160), "y".repeat(1)),
        ]);
        properties.sanitize();
        assert_eq!(4, properties.len());
        assert_eq!(8192, properties.get("1").unwrap().len());
        assert_eq!(8192, properties.get(&"2".repeat(150)).unwrap().len());
        assert_eq!(1, properties.get(&"3".repeat(150)).unwrap().len());
        assert_eq!("y", properties.get(&"4".repeat(150)).unwrap());
    }
}
