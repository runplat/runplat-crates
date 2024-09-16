mod put;
mod entity;
mod item;
mod observe;

use std::collections::BTreeMap;

pub use entity::Entity;
pub use item::Item;
pub use observe::ObservationEvent;

pub use put::Put;
use crate::{attribute::Attributes, Identifier, Repo, Resource};

/// Represents resources consumed by the application in a single map
/// 
/// ## Considerations
/// - When cloning a store directly, you capture a reference with the current mapped items. Although, the map
///   will not be updated, the items can be updated by a different owner.
#[derive(Clone)]
pub struct Store {
    /// Items in the store
    items: BTreeMap<u64, Item>,
    /// Repository of attributes for stored resources
    attrs: Repo<Attributes>,
}

impl Store {
    /// Returns a new store
    #[inline]
    pub fn new() -> Self {
        Self { attrs: Repo::new(), items: BTreeMap::new() }
    }

    /// Prepares to put a resource into the store
    #[must_use]
    #[inline]
    pub fn put<'a: 'b, 'b, R: Resource>(&'a mut self, resource: R) -> Put<'b, R> {
        let journal = self.attrs.journal.clone();
        Put {
            store: self,
            attributes: Attributes::new(journal),
            resource,
            ident: Identifier::Unit
        }
    }
}