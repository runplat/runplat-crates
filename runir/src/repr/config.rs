use super::*;

/// Struct to configure a table entry
pub struct Config<'a, R: Repr> {
    /// Mutable reference to a repr table
    pub(crate) repo: &'a mut Repo<R>,
    /// Handle
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Config<'a, R> {
    /// Applies a link identifier to the current handle
    #[inline]
    pub fn link(&mut self, link: impl Into<Identifier<'a>>) -> &mut Self {
        self.handle = self.handle.link_to(link.into());
        self
    }

    /// Intern a representation for the current handle
    /// 
    /// ## Considerations
    /// 
    /// This will replace any existing kind of representation for the current handles
    #[inline]
    pub fn intern(self, repr: R) -> (ReprHandle, Head<R>) {
        let mut head = Head::new(repr);
        head.journal = self.repo.journal.clone();
        let handle = head.inner.internals().handle().link_to(self.handle.link() as usize);
        self.repo
            .insert(handle.handle(), Kind::Interned(head.clone()));
        (handle, head)
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
            Kind::Interned({ 
                let mut head = Head::new(R::default());
                head.journal = self.repo.journal.clone();
                head
            }),
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
