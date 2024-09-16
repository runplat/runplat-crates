use crate::{Attributes, Identifier, Repo, Repr, ReprHandle, ReprInternals, Resource, TyRepr};

use super::{Item, Store};

pub struct Put<'put, R: Resource> {
    pub(crate) store: &'put mut Store,
    pub(crate) resource: R,
    pub(crate) attributes: Attributes,
    /// Identifier for this resource
    pub(crate) ident: Identifier<'put>,
}

impl<'put, R: Resource> Put<'put, R> {
    /// Adds an attribute for this resource
    #[inline]
    pub fn add<Attr: Repr>(&mut self, mut add: crate::repr::Add<'put, Attr>) -> &mut Self {
        add.ident(self.ident.clone());
        let repr = add.complete(&self.resource);
        self.attributes.attrs.insert(repr.handle(), repr.link());
        self
    }

    /// Applies an identifier for this resource
    #[inline]
    pub fn ident(&mut self, ident: impl Into<Identifier<'put>>) -> &mut Self {
        self.ident = ident.into();
        self
    }

    /// Commits the resource to the store
    #[must_use]
    pub fn commit(self) -> ReprHandle {
        // Complete adding the attribute for the resource
        let attrs = self.store.attrs.add(self.attributes);
        let handle = attrs.complete(&self.resource);
        self.store
            .items
            .insert(handle.link(), Item::from(self.resource));
        handle
    }
}

#[test]
fn test_put_resource_add_attr() {
    let mut store = Store::new();
    let mut repo = store.attrs.branch::<TyRepr>();

    let mut put = store.put(String::from("hello world"));

    put.add(repo.add(TyRepr::new::<u64>()));
    let id = put.commit();

    let handle = store.attrs.get(&id).unwrap();
    handle.get(id.clone(), Identifier::Unit);
    eprintln!("{id:?} -- {:?}", store.attrs.journal.logs());
    let casting = handle.interned();
    eprintln!("{:?}", casting.inner);

    let test = TyRepr::new::<TyRepr>();
    eprintln!("{:?}", test.handle());

    let t = casting.inner.get::<TyRepr>();
    eprintln!("{t:?}");
}
