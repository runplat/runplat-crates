use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::{atomic::AtomicUsize, Arc},
};

use crate::Resource;

use super::{Head, Repr, ReprHandle, ReprInternals};

/// Struct containing type information and an additional counter
pub struct TyRepr {
    /// Name of the type
    name: &'static str,
    /// Typeid value
    id: TypeId,
    /// Base hash value
    base_hash: u64,
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

impl Repr for TyRepr {
    fn link_hash_str(self: std::sync::Arc<Self>, ident: &str) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.base_hash.hash(&mut hasher);
        ident.hash(&mut hasher);
        hasher.finish()
    }
    
    fn link_hash_u64(self: Arc<Self>, identifier: u64) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        self.base_hash.hash(&mut hasher);
        identifier.hash(&mut hasher);
        hasher.finish()
    }
}

impl ReprInternals for TyRepr {
    fn handle_of(repr: super::Kind<Self>) -> ReprHandle {
        match repr {
            super::Kind::Internable(head) | super::Kind::Mappable { head, .. } => ReprHandle {
                handle: head.0.base_hash,
                shared: head.0.clone(),
                link: 0,
            },
        }
    }

    fn head(&self) -> Head<Self> {
        Head(
            TyRepr {
                name: self.name,
                id: self.id,
                base_hash: self.base_hash,
            }
            .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::TyRepr;
    use crate::{
        repr::{Kind, ReprInternals},
        Resource,
    };

    struct Test;

    impl Resource for Test {}

    #[test]
    fn test_ty_repr() {
        let test = Test;
        let repr = TyRepr::from(&test);
        let head = repr.head();

        let handle = TyRepr::handle_of(head.clone().into());
        assert_eq!(
            "557bc4be-fa08-80e3-0000-000000000000",
            handle.uuid().to_string()
        );
        let head = Kind::Internable(head);
        let mapped = head.map("test", TyRepr::from(&Test));

        let (handle, test) = mapped.get("test").expect("should exist");
        assert_eq!(repr.base_hash, test.base_hash);
        assert!(handle.link() > 0);
    }
}
