use crc::Crc;
use serde::Serialize;

use super::{Repr, ReprInternals};
use crate::Resource;
use std::{any::TypeId, sync::OnceLock};

static CRC: OnceLock<Crc<u64>> = OnceLock::new();

/// Returns a CRC algo for creating non-cryptographic hashes
pub fn crc() -> &'static Crc<u64> {
    CRC.get_or_init(|| Crc::<u64>::new(&crc::CRC_64_MS))
}

/// Struct containing type information
///
/// Also serves as the common ReprInternals implementation
#[derive(Debug, Serialize, PartialEq, PartialOrd)]
pub struct TyRepr {
    /// Name of the type
    name: &'static str,
    /// Typeid value
    #[serde(skip)]
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

        let mut digester = crc().digest();
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

impl ReprInternals for TyRepr {
    fn link_hash_str(&self, identifier: &str) -> u64 {
        let mut digester = crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        digester.update(identifier.as_bytes());
        digester.finalize()
    }

    fn link_hash_id(&self, identifier: usize) -> u64 {
        let mut digester = crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        digester.update(&identifier.to_be_bytes());
        digester.finalize()
    }

    fn link_hash<S: Serialize>(&self, digest: &S) -> u64 {
        let mut digester = crc().digest();
        digester.update(&self.base_hash.to_be_bytes());
        match bincode::serialize(digest) {
            Ok(b) => {
                digester.update(&b);
            }
            Err(_) => {}
        }
        digester.finalize()
    }
}
