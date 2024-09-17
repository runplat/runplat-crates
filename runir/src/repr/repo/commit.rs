use uuid::Uuid;

use super::*;

/// Data for executing a commit operation w/ a repo
///
/// A commit must create a new Head for repr data. It must first decide on a commit id to use.
/// This always starts at the current ReprInternals `hash_uuid`.
///
/// When completing the commit, the commit UUID is converted into a u64 value, by first creating a pair of u64 values from
/// the UUID, i.e. (hi, lo). And then performing a XOR on the pair of bits, i.e. commit = hi ^ lo.
///
/// The lo part of the commit UUID is used to uniquely identify this commit.
///
/// After, the commit is created a new head can be created and logged with the repo's journal, after which the handle can then be
/// returned back to the caller and the operation completes.
pub struct Commit<'op, R: Repr> {
    pub(super) repo: &'op mut Repo,
    pub(super) repr: R,
    pub(super) commit: uuid::Uuid,
}

impl<'op, R: Repr> Commit<'op, R> {
    /// Hashes the current repr into the current commit uuid
    #[inline]
    #[must_use = "Must call `finish()` to complete the operation"]
    pub fn digest_repr(mut self) -> Self
    where
        R: Content,
    {
        let internals = self.repr.internals();
        let next_lo = internals.link_hash_content(&self.repr);
        let (hi, lo) = self.commit.as_u64_pair();
        self.commit = Uuid::from_u64_pair(hi, lo ^ next_lo);
        self
    }

    /// Hashes some state into the current commit uuid
    #[inline]
    #[must_use = "Must call `finish()` to complete the operation"]
    pub fn digest<C: Content + ?Sized>(mut self, state: &C) -> Self {
        let internals = self.repr.internals();
        let next_lo = internals.link_hash_content(state);
        let (hi, lo) = self.commit.as_u64_pair();
        self.commit = Uuid::from_u64_pair(hi, lo ^ next_lo);
        self
    }

    /// Appends an ident to the current lo bits of the commit uuid
    #[inline]
    #[must_use = "Must call `finish()` to complete the operation"]
    pub fn ident(mut self, ident: impl Into<Identifier<'op>>) -> Self {
        let internals = self.repr.internals();
        let ident: Identifier = ident.into();
        let next_lo = match ident {
            Identifier::Unit => 0,
            Identifier::Str(cow) => internals.link_hash_str_id(&cow),
            Identifier::Id(id) => internals.link_hash_id(id),
        };
        let (hi, lo) = self.commit.as_u64_pair();
        self.commit = Uuid::from_u64_pair(hi, lo ^ next_lo);
        self
    }

    /// Consumes and performs the operation and returns the generated repo handle
    #[inline]
    pub fn finish(self) -> Handle {
        let (hi, lo) = self.commit.as_u64_pair();
        let commit = hi ^ lo;
        let head = Head::new(commit, self.repr);
        let handle = head.handle();
        self.repo.journal.log(handle.clone());
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit() {
        let mut repo = Repo::new();
        let commit = repo.commit(TyRepr::new::<String>());
        let handle = commit.finish();
        assert!(handle.cast::<TyRepr>().is_some());

        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TyRepr>().is_some());
    }

    #[test]
    fn test_commit_unit_ident() {
        let mut repo = Repo::new();
        let first = repo.commit(TyRepr::new::<String>()).finish();
        let handle = repo.commit(TyRepr::new::<String>()).ident(()).finish();
        assert_eq!(first.commit(), handle.commit());
        assert!(handle.cast::<TyRepr>().is_some());
        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TyRepr>().is_some());
    }

    #[test]
    fn test_commit_str_ident() {
        let mut repo = Repo::new();
        let first = repo.commit(TyRepr::new::<String>()).finish();
        let handle = repo
            .commit(TyRepr::new::<String>())
            .ident("hello world")
            .finish();
        assert_ne!(first.commit(), handle.commit());
        assert!(handle.cast::<TyRepr>().is_some());
        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TyRepr>().is_some());
    }

    #[test]
    fn test_commit_id_ident() {
        let mut repo = Repo::new();
        let first = repo.commit(TyRepr::new::<String>()).finish();
        let handle = repo
            .commit(TyRepr::new::<String>())
            .ident(3145usize)
            .finish();
        assert_ne!(first.commit(), handle.commit());
        assert!(handle.cast::<TyRepr>().is_some());
        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TyRepr>().is_some());
    }

    struct TestRepr {
        value: usize,
    }

    impl Content for TestRepr {
        fn state_uuid(&self) -> uuid::Uuid {
            uuid::Uuid::from_u64_pair(self.value as u64, 0)
        }
    }
    impl Resource for TestRepr {}
    impl Repr for TestRepr {}

    #[test]
    fn test_commit_hash_repr() {
        let mut repo = Repo::new();
        let first = repo.commit(TestRepr { value: 0 }).digest_repr().finish();
        let handle = repo.commit(TestRepr { value: 10 }).digest_repr().finish();
        assert_ne!(first.commit(), handle.commit());
        assert!(handle.cast::<TestRepr>().is_some());
        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TestRepr>().is_some());
    }

    #[test]
    fn test_commit_hash_state() {
        let mut repo = Repo::new();
        let first = repo
            .commit(TestRepr { value: 0 })
            .digest_repr()
            .digest(&"test123")
            .finish();
        let handle = repo
            .commit(TestRepr { value: 0 })
            .digest_repr()
            .digest(&"1234test")
            .finish();
        assert_ne!(first.commit(), handle.commit());
        assert!(handle.cast::<TestRepr>().is_some());
        let recalled = repo.checkout(handle.commit());
        assert!(recalled.is_some());
        assert!(recalled.unwrap().cast::<TestRepr>().is_some());
    }
}
