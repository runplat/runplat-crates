mod put;

pub use put::Put;
use crate::{attribute::Attributes, Repo, Resource};

pub struct Store {
    /// Repository of attributes for stored resources
    attrs: Repo<Attributes>
}

impl Store {
    /// Returns a new store
    pub fn new() -> Self {
        Self { attrs: Repo::new() }
    }

    /// Prepares to put a resource into the store
    pub fn put<'a: 'b, 'b, R: Resource>(&'a mut self, resource: R) -> Put<'b, R> {
        let journal = self.attrs.journal.clone();
        Put {
            store: self,
            attributes: Attributes::new(journal),
            resource
        }
    }
}