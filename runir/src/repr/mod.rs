mod attribute;
mod labels;
pub mod repo;
mod ty;
use crate::{Content, Resource};
pub use attribute::Attributes;
pub use labels::Labels;
pub use repo::Repo;
use std::{any::TypeId, borrow::Cow, fmt::Debug, pin::Pin, sync::Arc};
pub use ty::TyRepr;

/// Enumeration of identifier variants
#[derive(Clone)]
pub enum Identifier<'a> {
    Unit,
    Str(Cow<'a, str>),
    Id(usize),
}

/// Representation is associated data that can be used to represent a resource in various contexts
///
/// For example, a resource's type information is it's representation within a rust application.
pub trait Repr: Resource {
    /// Returns the representation internals
    fn internals(&self) -> impl ReprInternals
    where
        Self: Sized,
    {
        // As the default implementation for the internals the ty_repr returns a hash_uuid
        // that at the base case returns a hashed identifier of the current rust type information
        // at the hi bits, w/ the lower-bits zeroed out
        TyRepr::new::<Self>()
    }
}

/// Representation internals required for managing repr maps and tables
pub trait ReprInternals: Sized + Repr {
    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_str_id(&self, identifier: &str) -> u64;

    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_id(&self, identifier: usize) -> u64;

    /// Returns a "link" value of a representation instance after "hashing" an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_content<C: Content + ?Sized>(&self, content: &C) -> u64;

    /// Returns a uuid that can be used for hashing
    fn hash_uuid<T>(&self) -> uuid::Uuid
    where
        T: Sized,
    {
        let hi = self.link_hash_id(0);
        uuid::Uuid::from_u64_pair(hi, 0)
    }
}

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(Cow::from(value))
    }
}

impl<'a> From<usize> for Identifier<'a> {
    fn from(value: usize) -> Self {
        Identifier::Id(value)
    }
}

impl<'a> From<u64> for Identifier<'a> {
    fn from(value: u64) -> Self {
        Identifier::Id(value as usize)
    }
}

impl<'a> From<()> for Identifier<'a> {
    fn from(_: ()) -> Self {
        Identifier::Unit
    }
}
