use std::{collections::BTreeMap, ops::Deref};

use crate::Resource;

use super::Repr;

/// Wrapper struct for a ordered label representation
#[derive(Hash)]
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
}
