use super::*;

/// Struct for storing data for an "add" operation w/ `store.put`
///
/// ```norun
/// let store = Store::new();
///
/// let label = Repo::new::<Label>();
/// let new_label = label.add(["name", "hello-world"]);
///
/// store.put(MyResource)
///     .add(new_label)
///     .commit();
/// ```
pub struct Add<'a, R: Repr> {
    /// Repo to add the repr to
    pub(super) repo: &'a mut Repo<R>,
    /// Repr to associate to a resource
    pub(super) repr: R,
    /// Identifier to apply when adding this repr
    pub(super) ident: Identifier<'a>,
}

impl<'a, R: Repr> Add<'a, R> {
    /// Sets the identifier to use when associating this representation with a resource
    #[inline]
    pub fn ident(&mut self, ident: impl Into<Identifier<'a>>) -> &mut Self {
        self.ident = ident.into();
        self
    }

    /// Consumes this object and completes adding the repr for a res
    #[inline]
    pub fn complete<Res: Resource>(self, res: &Res) -> ReprHandle {
        let mut config = self.repo.config(res);
        config.link(self.ident.clone());
        let (handle, _) = config.intern(self.repr);
        handle
    }
}
