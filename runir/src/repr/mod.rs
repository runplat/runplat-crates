mod ty;
use std::{collections::BTreeMap, sync::{Arc, Weak}};

use crate::Key;

/// Struct for a global table storing handles that point to representation data
pub struct ReprTable<R: Repr> {
    /// Thread-safe map of repr handles that map to a repr
    map: ReprMap<R>
}

/// Representation is associated data that can be used to represent a resource in various contexts
/// 
/// For example, a resource's type information is it's representation within a rust application.
pub trait Repr: Sized + Send + Sync + 'static {
    /// Returns the head value of this representation
    fn head(&self) -> Head<Self>;

    /// Returns the "handle" value of a representation instance
    fn handle_of(repr: Kind<Self>) -> u64;

    /// Returns a new "ident" value of a representation instance given an identifier
    /// 
    /// **Note**: This is not a hashing function
    fn create_ident(head: Head<Self>, identifier: impl std::hash::Hash) -> u64;
}

/// Struct containing the head representation value
pub struct Head<R>(pub Arc<R>);

impl<R> Clone for Head<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Repr> From<Head<R>> for Kind<R> {
    fn from(value: Head<R>) -> Self {
        Self::Internable(value)
    }
}

/// Handle containing key data to a specific representation
struct ReprHandle {
    handle: u64,
    ident: u64
}

struct ReprMap<R: Repr> {
    inner: BTreeMap<u64, Kind<R>>
}

type OrderedReprMap<R> = BTreeMap<u64, Arc<R>>;

/// Enumeration of kinds of representations
pub enum Kind<R: Repr> {
    /// Internable representations are typically constant literal values that can be broadly represented,
    /// such as type information derived by the compiler. It is well suited for counting classification, but
    /// not well suited for representations that are unique per resource.
    Internable(Head<R>),
    /// Mappable representation where each resource can map to a unique repr value. This kind of representation requires an additional "next" value
    /// in addition to the "handle" value in order to succesfully store. It is well suited for naming resources or attaching other types of
    /// labeling data
    Mappable {
        /// Head value which can be used to derive identifier keys for mapped representations
        head: Head<R>,
        /// Thread-safe conccurrent ordered map of identified representations
        map: OrderedReprMap<R>
    }
}

impl<R: Repr> Kind<R> {
    /// Returns true if this representaion is internable
    pub fn is_internable(&self) -> bool {
        matches!(self, Kind::Internable { .. })
    }

    /// Returns true if this representation is mappable
    pub fn is_mappable(&self) -> bool {
        matches!(self, Kind::Mappable { .. })
    }
}

/// Enumeration of views of representations
pub enum View<R: Repr> {
    /// View of the value of an interned representation value
    Interned {
        head: Weak<R>
    },
    /// View of the value of an interned mapped value
    Mapped {
        ident: u64,
        value: Weak<R>
    }
}