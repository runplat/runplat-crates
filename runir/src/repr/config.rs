use super::*;

/// Struct to configure a table entry
pub struct Config<'a, R: Repr> {
    /// Mutable reference to a repr table
    pub(crate) repo: &'a mut Repo<R>,
    /// Handle
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Config<'a, R> {
    /// Intern a representation for the current handle
    #[inline]
    pub fn intern(self, repr: R) {
        self.repo
            .insert(self.handle.handle(), Kind::Interned(Head::new(repr)));
    }

    /// Returns a Map config builder for mapping an identifier to a represntation for the current handle
    ///
    /// If an interned value does not currently exist, the default head value will be used
    #[inline]
    pub fn default_mapped<'new>(self) -> Map<'new, R>
    where
        'a: 'new,
        R: Default,
    {
        self.repo.insert(
            self.handle.handle(),
            Kind::Interned(Head::new(R::default())),
        );

        self.mapped()
    }

    /// Returns a Map config builder for mapping an identifier to a represntation for the current handle
    #[inline]
    pub fn mapped<'new>(self) -> Map<'new, R>
    where
        'a: 'new,
    {
        Map {
            repo: self.repo,
            handle: self.handle,
        }
    }
}
