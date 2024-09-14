use super::*;

/// Struct to configure a table entry
pub struct Config<'a, R: Repr> {
    /// Mutable reference to a repr table
    pub(crate) table: &'a mut ReprTable<R>,
    /// Handle
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Config<'a, R> {
    /// Intern a representation for the current handle
    #[inline]
    pub fn intern(self, repr: R)
    where
        R: ReprInternals,
    {
        self.table
            .tree
            .inner
            .insert(self.handle.handle(), Kind::Internable(Head(repr.into())));
    }

    /// Returns a Map config builder for mapping an identifier to a represntation for the current handle
    ///
    /// If an interned value does not currently exist, the default head value will be used
    #[inline]
    pub fn default_mapped<'new>(self) -> Map<'new, R>
    where
        'a: 'new,
        R: ReprInternals + Default,
    {
        self.table
            .tree
            .inner
            .entry(self.handle.handle())
            .or_insert_with(|| Kind::Internable(R::default().head()));

        self.mapped()
    }
    
    /// Returns a Map config builder for mapping an identifier to a represntation for the current handle
    #[inline]
    pub fn mapped<'new>(self) -> Map<'new, R>
    where
        'a: 'new,
        R: ReprInternals,
    {
        Map {
            table: self.table,
            handle: self.handle,
        }
    }
}
