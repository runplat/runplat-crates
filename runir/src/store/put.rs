use crate::{Attributes, Repo, Repr, ReprInternals, Resource};

use super::Store;

pub struct Put<'put, R: Resource> {
    pub(crate) store: &'put mut Store,
    pub(crate) resource: R,
    pub(crate) attributes: Attributes,
}

impl<'put, R: Resource> Put<'put, R> {
    /// Adds an attribute for this resource
    pub fn add<Attr: Repr>(&mut self, repr: Attr, repo: &mut Repo<Attr>) -> &mut Self {
        let res_ty = crate::TyRepr::new::<R>();
        let head = crate::Head::new(repr);
        let handle = head.0.internals().handle();
        let handle = handle
            .link_to(res_ty.handle().handle() as usize)
            .checkout(head.clone(), self.store.attrs.journal.clone());
        repo.insert(handle.handle(), head);
        self.attributes.attrs.insert(handle.handle(), handle.link());
        self
    }

    /// Commits the resource to the store
    #[must_use]
    pub fn commit(self) {
        self.store
            .attrs
            .config(&self.resource)
            .intern(self.attributes);
    }
}

#[test]
fn test() {
    let l = std::sync::RwLock::new(10);

    let guard = match l.write() {
        Ok(v) => v,
        Err(e) => e.into_inner(),
    };
}
