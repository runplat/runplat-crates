use super::*;

/// Finds a representation from a table
pub struct Find<'a, R: Repr> {
    pub(crate) table: &'a ReprTable<R>,
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Find<'a, R> {
    /// Consumes the config to search for a representation that maps to an identifier
    #[inline]
    pub fn ident<'b>(self, ident: impl Into<Identifier<'b>>) -> Option<&'b Arc<R>>
    where
        'a: 'b,
        R: ReprInternals,
    {
        self.table
            .tree
            .get(self.handle)
            .and_then(|e| e.get(ident))
            .map(|(_, v)| v)
    }

    /// Searches for the interned value if it exists
    #[inline]
    pub fn interned<'b>(self) -> Option<&'b Arc<R>>
    where
        'a: 'b,
        R: ReprInternals,
    {
        self.table
            .tree
            .get(self.handle)
            .map(|e| e.head())
            .map(|h| &h.0)
    }
}
