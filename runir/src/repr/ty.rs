use super::{Repr, ReprInternals};
use crate::{Content, Resource};
use std::any::TypeId;

/// Struct containing type information
///
/// Also serves as the common ReprInternals implementation
#[derive(Debug, PartialEq, PartialOrd)]
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

        let mut digester = crate::content::crc().digest();
        digester.update(name.as_bytes());
        Self {
            name,
            id,
            base_hash: digester.finalize(),
        }
    }
}

impl Repr for TyRepr {}
impl Resource for TyRepr {}

impl Content for TyRepr {
    fn state_uuid(&self) -> uuid::Uuid {
        uuid::Uuid::from_u64_pair(self.base_hash, 0)
    }
}

impl ReprInternals for TyRepr {
    fn link_hash_str_id(&self, identifier: &str) -> u64 {
        let mut digester = crate::content::crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        digester.update(identifier.as_bytes());
        digester.finalize()
    }

    fn link_hash_id(&self, identifier: usize) -> u64 {
        let mut digester = crate::content::crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        digester.update(&identifier.to_be_bytes());
        digester.finalize()
    }

    fn link_hash_content<C: Content + ?Sized>(&self, digest: &C) -> u64 {
        let mut digester = crate::content::crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        digester.update(digest.state_uuid().as_bytes());
        digester.finalize()
    }
}
