use super::*;

/// Struct for a repo storing representation data for objects
#[derive(Clone)]
pub struct Repo<R: Repr> {
    /// Tree storing representations for this table
    tree: BTreeMap<u64, Kind<R>>,
    /// Journal storing repr's that have been checked out
    pub(crate) journal: Journal,
}

impl<R: Repr> Repo<R> {
    /// Creates a new repr table
    ///
    /// A Repr table stores representations and consists of the main tree which maps to each representation,
    /// and a shared "Keys" reference which is used to generate normalize lookup keys
    #[inline]
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(),
            journal: Journal::new(),
        }
    }

    /// Creates a new repo which stores different representations, sharing the same
    /// journal
    #[inline]
    pub fn branch<O: Repr>(&self) -> Repo<O> {
        let mut repo = Repo::new();
        repo.journal = self.journal.clone();
        repo
    }

    /// Checkout a representation from this table for a resource
    #[inline]
    pub fn checkout<Res: Resource>(&self, resource: &Res) -> Checkout<'_, R> {
        let handle = self.type_handle(resource);
        Checkout {
            repo: self,
            handle,
        }
    }

    /// Configures a representation from this table for a resource
    #[inline]
    pub fn config<Res: Resource>(&mut self, resource: &Res) -> Config<'_, R> {
        let handle = self.type_handle(resource);
        Config {
            repo: self,
            handle,
        }
    }

    /// Begins an add operation for use with a `Store`
    #[inline]
    #[must_use = "When adding a representation for a resource, the output of this function must be used in conjunction with Store::put(..)"]
    pub fn add(&mut self, repr: R) -> Add<'_, R> {
        Add {
            repo: self,
            repr,
            ident: Identifier::Unit
        }
    }

    /// Resolves a link into a handle by searching journaled logs
    /// 
    /// Returns None if the representation has never been accessed
    #[inline]
    pub fn resolve(&self, link: u64) -> Option<ReprHandle> {
        self.journal.logs().get(&link).cloned()
    }

    /// Returns a reference for a node searching by handle
    #[inline]
    pub fn get(&self, handle: &ReprHandle) -> Option<&Kind<R>> {
        self.tree.get(&handle.handle())
    }

    /// Returns a mutable reference for a node searching by handle
    #[inline]
    pub fn get_mut(&mut self, handle: &ReprHandle) -> Option<&mut Kind<R>> {
        self.tree.get_mut(&handle.handle())
    }

    /// Inserts a new node directly into the inner tree
    #[inline]
    pub(crate) fn insert(&mut self, handle: u64, node: impl Into<Kind<R>>) {
        self.tree.insert(handle, node.into());
    }

    /// Returns a type handle for a resource
    #[inline]
    fn type_handle<Res: Resource>(&self, resource: &Res) -> ReprHandle {
        let ty_repr = TyRepr::from(resource);
        ty_repr.handle()
    }
}

impl<R: Repr> Default for Repo<R> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug)]
    struct TestRepr;
    impl Resource for TestRepr {}
    impl Repr for TestRepr {}

    struct TestResource;
    impl Resource for TestResource {}

    #[test]
    fn test_repr_table() {
        let mut table = Repo::<TestRepr>::new();
        table.config(&TestResource).intern(TestRepr);
        table.config(&TestResource).mapped().map("test", TestRepr);

        let _ = table
            .checkout(&TestResource)
            .ident("test")
            .expect("should exist");

        let test_string = String::from("hello world");
        let test_string2 = String::from("hello world");
        table
            .config(&test_string)
            .default_mapped()
            .map("test2", TestRepr);

        let (handle, _) = table
            .checkout(&test_string)
            .ident("test2")
            .expect("should exist");

        let is_some = table.checkout(&test_string2).ident("test2").is_some();
        assert!(is_some);

        let repr = TyRepr::from(&test_string);
        let _repr = repr.handle().cast::<TyRepr>();
        assert!(_repr.is_some());
        let _repr = repr.handle().cast::<TyRepr>();
        assert!(_repr.is_some());

        let repr = TyRepr::from(&test_string);
        let repr = repr.handle().cast::<TestRepr>();
        assert!(repr.is_none());

        let link = handle.link();
        eprintln!("{:?}", link);
        let repr = table.resolve(link);
        eprintln!("{:?}", repr.is_some());
    }
}
