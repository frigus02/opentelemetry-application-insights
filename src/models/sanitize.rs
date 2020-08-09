use std::collections::BTreeMap;

pub(crate) trait Sanitize {
    fn sanitize(&mut self);
}

impl Sanitize for BTreeMap<String, String> {
    fn sanitize(&mut self) {
        let keys: Vec<_> = self
            .keys()
            .filter(|k| k.len() > 150)
            .map(|k| k.to_owned())
            .collect();
        for mut key in keys {
            let value = self
                .remove(&key)
                .expect("value needs to exist. got key by iterating over map");
            key.truncate(150);
            self.insert(key, value);
        }
        for value in self.values_mut() {
            value.truncate(8192);
        }
    }
}
