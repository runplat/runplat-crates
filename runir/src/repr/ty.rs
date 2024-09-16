use super::{Head, Repr, ReprHandle, ReprInternals};
use crate::Resource;
use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Struct containing type information
///
/// Also serves as the common ReprInternals implementation
#[derive(Debug)]
pub struct TyRepr {
    /// Name of the type
    name: &'static str,
    /// Typeid value
    id: TypeId,
    /// Base hash value
    base_hash: u64,
}

impl TyRepr {
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

    fn head(&self) -> Head<Self> {
        Head::new(TyRepr {
            name: self.name,
            id: self.id,
            base_hash: self.base_hash,
        })
    }

    fn handle(&self) -> ReprHandle {
        ReprHandle {
            handle: self.base_hash,
            head: self.head().inner.clone(),
            link: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TyRepr;
    use crate::{
        repr::{journal::Journal, Kind, ReprInternals},
        Resource,
    };

    struct Test;

    impl Resource for Test {}

    #[test]
    fn test_ty_repr() {
        let test = Test;
        let repr = TyRepr::from(&test);
        let head = repr.head();

        let handle = head.inner.handle();
        assert!(handle.handle() > 0);
        let head = Kind::Interned(head);
        let mapped = head.map(handle.clone(), "test", TyRepr::from(&Test));

        let (handle, test) = mapped.get(handle, "test").expect("should exist");
        assert_eq!(repr.base_hash, test.inner.base_hash);
        assert!(handle.link() > 0);
    }
}
