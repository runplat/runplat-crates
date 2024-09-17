use super::*;
use crate::{repo::Handle, repr::Repr};
use serde::Serialize;

/// Constructs a "put" operation to the store
pub struct Put<'put, R> {
    /// Store being modified
    pub(crate) store: &'put mut Store,
    /// Resource being put into the store
    pub(crate) resource: R,
    /// Attributes map for this resource
    pub(crate) attributes: Attributes,
    /// Identifier for this resource
    pub(crate) ident: Identifier<'put>,
}

impl<'put, R: Resource + Serialize> Put<'put, R> {
    /// Returns a refernce to the resource being put into the srore
    #[inline]
    pub fn resource(&self) -> &R {
        &self.resource
    }

    /// Returns a mutable reference to the resource being put into the store
    #[inline]
    pub fn resource_mut(&mut self) -> &mut R {
        &mut self.resource
    }

    /// Adds an attribute for this resource
    #[inline]
    pub fn attr<Attr: Repr + Serialize>(mut self, attr: Attr) -> Self {
        let handle = self
            .store
            .repo
            .assign(attr, &self.resource)
            .ident(self.ident.clone())
            .complete();
        self.attributes.insert::<Attr>(&handle);
        self
    }

    /// Applies an identifier for this resource
    #[inline]
    pub fn ident(&mut self, ident: impl Into<Identifier<'put>>) -> &mut Self {
        self.ident = ident.into();
        self
    }

    /// Commits the resource to the store
    #[inline]
    #[must_use]
    pub fn commit(self) -> Handle {
        let handle = self
            .store
            .repo
            .assign(self.attributes, &self.resource)
            .ident(self.ident.clone())
            .complete();

        self.store.items.insert(
            handle.commit(),
            Item::new(
                self.store.repo.journal.clone(),
                handle.commit(),
                self.resource,
            ),
        );
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repr::TyRepr;

    #[test]
    fn test_put_resource_add_attr() {
        let mut store = Store::new();

        let handle = store
            .put(String::from("hello world"))
            .attr(TyRepr::new::<u64>())
            .commit();

        let attributes = handle.cast::<Attributes>().expect("should have attributes");
        let ty_repr = attributes.get::<TyRepr>().expect("should have a ty_repr");
        assert_eq!(ty_repr.as_ref(), &TyRepr::new::<u64>())
    }
}
