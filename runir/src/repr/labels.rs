use super::Repr;
use crate::{Content, Resource};
use std::{collections::BTreeMap, ops::Deref};

/// Wrapper struct for a ordered label representation
pub struct Labels(pub BTreeMap<String, String>);

impl Repr for Labels {}
impl Resource for Labels {}

impl From<&[(&str, &str)]> for Labels {
    fn from(value: &[(&str, &str)]) -> Self {
        let mut map = BTreeMap::new();
        for (k, v) in value {
            map.insert(k.to_string(), v.to_string());
        }
        Labels(map)
    }
}

impl From<BTreeMap<String, String>> for Labels {
    fn from(value: BTreeMap<String, String>) -> Self {
        Labels(value)
    }
}

impl Deref for Labels {
    type Target = BTreeMap<String, String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Content for Labels {
    fn state_uuid(&self) -> uuid::Uuid {
        let mut crc = crate::content::crc().digest();
        for (k, v) in self.0.iter() {
            crc.update(k.as_bytes());
            crc.update(v.as_bytes());
        }
        uuid::Uuid::from_u64_pair(crc.finalize(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repr::Attributes, Store};

    #[test]
    fn test_labels() {
        let mut store = Store::new();

        let handle = store
            .put(String::from("hello world"))
            .attr(Labels::from(
                &[("name", "random string"), ("media-type", "rust string")][..],
            ))
            .commit();

        assert!(handle.cast::<Attributes>().is_some());
        let attributes = handle.cast::<Attributes>().unwrap();

        let labels = attributes.get::<Labels>().unwrap();
        assert_eq!("random string", labels.get("name").unwrap());
        assert_eq!("rust string", labels.get("media-type").unwrap());
    }

    #[test]
    fn test_labels_from_btree() {
        let mut store = Store::new();
        let mut labels = BTreeMap::new();
        labels.insert("name".to_string(), "random string".to_string());
        labels.insert("media-type".to_string(), "rust string".to_string());

        let handle = store
            .put(String::from("hello world"))
            .attr(Labels::from(labels))
            .commit();

        assert!(handle.cast::<Attributes>().is_some());
        let attributes = handle.cast::<Attributes>().unwrap();

        let labels = attributes.get::<Labels>().unwrap();
        assert_eq!("random string", labels.get("name").unwrap());
        assert_eq!("rust string", labels.get("media-type").unwrap());
    }
}
