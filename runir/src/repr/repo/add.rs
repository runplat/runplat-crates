use super::*;

/// Struct for storing addressable representation data for a resource within a repo
///
pub struct Add<'a, R: Repr + Hash, Rx: Resource + Hash> {
    /// Repo to add the repr to
    pub(super) commit: Commit<'a, R>,
    pub(super) resource: &'a Rx,
}

impl<'a, R: Repr + Hash, Rx: Resource + Hash> Add<'a, R, Rx> {
    /// Sets the identifier to use when associating this representation with a resource
    #[inline]
    pub fn ident(mut self, ident: impl Into<Identifier<'a>>) -> Self {
        self.commit = self.commit.ident(ident);
        self
    }

    /// Consumes this object and completes adding the repr for a res
    #[inline]
    pub fn complete(mut self) -> Handle {
        self.commit = self.commit.hash_repr().hash(self.resource);
        self.commit.finish()
    }
}
