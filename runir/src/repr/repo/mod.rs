mod add;
mod commit;
mod handle;
mod head;
mod journal;

pub use add::Add;
pub use commit::Commit;
pub use handle::Handle;
pub use journal::Journal;

use super::*;
use head::Head;

/// Struct for a repo storing representation data for objects
#[derive(Clone)]
pub struct Repo {
    /// Journal storing repr's that have been checked out
    pub(crate) journal: Journal,
}

impl Repo {
    /// Creates a new repr table
    ///
    /// A Repr table stores representations and consists of the main tree which maps to each representation,
    /// and a shared "Keys" reference which is used to generate normalize lookup keys
    #[inline]
    pub fn new() -> Self {
        Self {
            journal: Journal::new(),
        }
    }

    /// Prepares to commit a new representation to the current repo
    ///
    /// Commits a representation to the current repo
    #[inline]
    #[must_use]
    pub fn commit<R: Repr>(&mut self, repr: R) -> Commit<'_, R> {
        let internals = repr.internals();
        Commit {
            repo: self,
            repr,
            commit: internals.hash_uuid::<R>(),
        }
    }

    /// Returns a committed representation stored in this repo
    #[inline]
    pub fn checkout(&self, commit: u64) -> Option<Handle> {
        self.journal.get(commit)
    }

    /// Begins an assign operation for use with a `Store`
    ///
    /// Assign will assign an attribute that implements Repr to a resource. Both must be hashable. The assignment is unique to the specific repr.
    #[inline]
    #[must_use = "When adding a representation for a resource, the output of this function must be used in conjunction with Store::put(..)"]
    pub fn assign<'a: 'b, 'b, R: Repr + Hash, Rx: Resource + Hash>(
        &'a mut self,
        repr: R,
        resource: &'a Rx,
    ) -> Add<'b, R, Rx> {
        Add {
            commit: self.commit(repr),
            resource,
        }
    }
}

impl Default for Repo {
    fn default() -> Self {
        Self::new()
    }
}
