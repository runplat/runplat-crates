use super::*;

/// Checkout a representation from a repo
pub struct Checkout<'a, R: Repr> {
    /// Repo being searched
    pub(crate) repo: &'a Repo<R>,
    /// Handle
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Checkout<'a, R> {
    /// Consumes the config to search for a representation that maps to an identifier
    #[inline]
    pub fn ident<'b>(self, ident: impl Into<Identifier<'b>>) -> Option<(ReprHandle, &'b Head<R>)>
    where
        'a: 'b,
    {
        self.repo
            .get(&self.handle)
            .and_then(|e| e.get(self.handle.clone(), ident))
    }

    /// Searches for the interned value if it exists
    #[inline]
    pub fn interned<'b>(self) -> Option<&'b Head<R>>
    where
        'a: 'b,
    {
        self.repo.get(&self.handle).map(|e| e.interned())
    }
}
