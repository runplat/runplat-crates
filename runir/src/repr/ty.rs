use std::{
    any::TypeId,
    hash::{Hash, Hasher},
    sync::atomic::AtomicUsize,
};

use crate::Resource;

use super::{Head, Repr};

/// Struct containing type information and an additional counter
pub struct TyRepr {
    /// Name of the type
    name: &'static str,
    /// Typeid value
    id: TypeId,
    /// Base hash value
    base_hash: u64,
    /// Counter
    counter: std::sync::atomic::AtomicUsize,
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
            counter: AtomicUsize::new(0),
        }
    }
}

impl Repr for TyRepr {
    fn head(&self) -> Head<Self> {
        Head(
            TyRepr {
                name: self.name,
                id: self.id,
                base_hash: self.base_hash,
                counter: AtomicUsize::new(0),
            }
            .into(),
        )
    }

    fn handle_of(repr: super::Kind<Self>) -> u64 {
        match repr {
            super::Kind::Internable(head) => {
                head.0.base_hash
            },
            super::Kind::Mappable { head, .. } => head.0.base_hash,
        }
    }

    fn create_ident(
        head: super::Head<Self>,
        ident: impl std::hash::Hash,
    ) -> u64 {
        let mut hasher = std::hash::DefaultHasher::new();
        let next = head.0.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        next.hash(&mut hasher);
        ident.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::{repr::Repr, Resource};
    use super::TyRepr;

    struct Test;

    impl Resource for Test {}

    #[test]
    fn test_ty_repr() {
        let test = Test;
        let repr = TyRepr::from(&test);
        let head = repr.head();

        let handle = TyRepr::handle_of(head.clone().into());
        assert_eq!(head.0.base_hash, handle);

        let t1 = TyRepr::create_ident(head.clone(), "t1");
        let t2 = TyRepr::create_ident(head.clone(), "t2");
        let t1_other = TyRepr::create_ident(head, "t1");
        assert_ne!(t1, t2);
        assert_ne!(t1, t1_other);
    }
}