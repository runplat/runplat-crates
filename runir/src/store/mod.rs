mod put;
mod item;
mod observe;

use std::collections::BTreeMap;

pub use item::Item;

pub use observe::ObservationEvent;

pub use put::Put;

use crate::{repr::{Attributes, Identifier, Repo}, Resource};

/// Represents resources consumed by the application in a single map
/// 
/// ## Considerations
/// - When cloning a store directly, you capture a reference with the current mapped items. Although, the map
///   will not be updated, the items can be updated by a different owner.
#[derive(Clone)]
pub struct Store {
    /// Items in the store
    items: BTreeMap<u64, Item>,
    /// Repository of resource representation data
    repo: Repo,
}

impl Store {
    /// Returns a new store
    #[inline]
    pub fn new() -> Self {
        Self { repo: Repo::new(), items: BTreeMap::new() }
    }

    /// Prepares to put a resource into the store
    #[must_use]
    #[inline]
    pub fn put<'a: 'b, 'b, R: Resource>(&'a mut self, resource: R) -> Put<'b, R> {
        let journal = self.repo.journal.clone();
        Put {
            store: self,
            resource,
            ident: Identifier::Unit,
            attributes: Attributes::new(journal)
        }
    }

    /// Returns an item in the store mapped to the commit id
    #[inline]
    pub fn item(&self, commit: u64) -> Option<&Item> {
        self.items.get(&commit)
    }
}