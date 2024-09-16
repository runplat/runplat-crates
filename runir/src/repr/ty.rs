use super::{Repr, ReprInternals};
use crate::Resource;
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
};

/// Struct containing type information
///
/// Also serves as the common ReprInternals implementation
#[derive(Debug, Hash, PartialEq, PartialOrd)]
pub struct TyRepr {
    /// Name of the type
    name: &'static str,
    /// Typeid value
    id: TypeId,
    /// Base hash value
    base_hash: u64,
}

impl TyRepr {
    /// Creates a new type representation without a value
    #[inline]
    pub fn new<T: Resource>() -> Self {
        let name = std::any::type_name::<T>();
        let id = std::any::TypeId::of::<T>();

        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        id.hash(&mut hasher);
        Self {
            name,
            id,
            base_hash: hasher.finish(),
        }
    }
}

impl<R: Resource> From<&R> for TyRepr {
    fn from(value: &R) -> Self {
        let name = std::any::type_name_of_val(value);
        let id = value.type_id();

        let mut hasher = std::hash::DefaultHasher::new();
        name.hash(&mut hasher);
        id.hash(&mut hasher);
        Self {
            name,
            id: value.type_id(),
            base_hash: hasher.finish(),
        }
    }
}

impl Repr for TyRepr {}
impl Resource for TyRepr {}

impl ReprInternals for TyRepr {
    fn link_hash_str(&self, identifier: &str) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.base_hash.hash(&mut hasher);
        identifier.hash(&mut hasher);
        hasher.finish()
    }

    fn link_hash_id(&self, identifier: usize) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.base_hash.hash(&mut hasher);
        identifier.hash(&mut hasher);
        hasher.finish()
    }

    fn link_hash(&self, hash: impl Hash) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.base_hash.hash(&mut hasher);
        hash.hash(&mut hasher);
        hasher.finish()
    }
}
