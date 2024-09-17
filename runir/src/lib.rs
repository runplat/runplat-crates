//! # Runtime Intermediate Representation
//!
//! This library is for building an runtime intermediate representations of resources and their associated representations consumed during runtime.
//!
//! The three main modules are `store`, `repo`, and `repr`.
//!
//! The entrypoint to use this library is the "Store" type. The store centralizes resource state in a single location returning "Handles" that can be used to fetch
//! metadata for stored resources.
//!
//! The important thing is that the returned handle will always have a unique "commit" id which is a u64 value that can be passed around as a key to the resource.

pub mod repr;
pub use repr::repo;
pub mod store;
pub use repr::Repr;
pub use store::Store;

/// Trait representing a dynamic resource which can be stored and retrieved
pub trait Resource: std::any::Any + Send + Sync + 'static {}

/// Trait indicating that the implementing type is "Content" that can be converted into "state"
pub trait Content {
    /// Returns a UUID that represents the current content state
    /// 
    /// Implementing type must uphold the invariant, that immutable content must always return the same state_uuid.
    fn state_uuid(&self) -> uuid::Uuid;
}

impl Resource for String {}
impl Resource for bool {}
impl Resource for u128 {}
impl Resource for u64 {}
impl Resource for u32 {}
impl Resource for usize {}
impl Resource for f64 {}
impl Resource for f32 {}

pub mod content {
    use std::sync::OnceLock;
    use crc::Crc;
    use super::*;

    static CRC: OnceLock<Crc<u64>> = OnceLock::new();

    /// Returns a CRC algo for creating non-cryptographic hashes
    pub fn crc() -> &'static Crc<u64> {
        CRC.get_or_init(|| Crc::<u64>::new(&crc::CRC_64_MS))
    }
    
    impl<'a> Content for &'a str {
        fn state_uuid(&self) -> uuid::Uuid {
            let mut crc = crc().digest();
            crc.update(self.as_bytes());
            uuid::Uuid::from_u64_pair(crc.finalize(), 0)
        }
    }
    
    impl<'a> Content for String {
        fn state_uuid(&self) -> uuid::Uuid {
            let mut crc = crc().digest();
            crc.update(self.as_bytes());
            uuid::Uuid::from_u64_pair(crc.finalize(), 0)
        }
    }
}
