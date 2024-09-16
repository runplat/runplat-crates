use super::Handle;
use crate::Resource;
use std::{
    collections::BTreeMap,
    ops::Deref,
    sync::{Arc, Mutex},
};

/// Type-alias for the main synchronization primative
type SyncContext = Arc<Mutex<LogState>>;

/// Journal containing metadata to lookup representations that have been checked out
#[derive(Clone)]
pub struct Journal {
    /// Thread-safe log container
    log: Log,
}

impl Journal {
    /// Creates a new journal
    #[inline]
    pub fn new() -> Self {
        Self { log: Log::new() }
    }

    /// Log a handle to an ident and return the "link" value
    #[inline]
    pub fn log(&self, handle: Handle) -> u64 {
        self.log.record(&handle)
    }

    /// Returns a repr handle mapped to a link value
    ///
    /// Returns None if the handle has not been journaled
    #[inline]
    pub fn get(&self, link: u64) -> Option<Handle> {
        self.log.snapshot().get(&link).cloned()
    }

    /// Returns a snapshot of the underlying logs
    #[inline]
    pub fn logs(&self) -> Arc<BTreeMap<u64, Handle>> {
        self.log.snapshot().0.clone()
    }
}

/// Wrapper-struct for a thread-safe logging interface
#[derive(Clone, Debug)]
pub struct Log {
    sync: SyncContext,
}

#[derive(Default, Debug)]
struct LogState {
    snapshot: LogSnapshot,
    recorded: BTreeMap<u64, Handle>,
}

#[derive(Default, Debug, Clone)]
struct LogSnapshot(Arc<BTreeMap<u64, Handle>>);

impl Log {
    /// Creates a new log
    #[inline]
    fn new() -> Self {
        Log {
            sync: SyncContext::default(),
        }
    }

    /// Records a handle and returns the link value that can be used for later retrieval
    #[inline]
    fn record(&self, handle: &Handle) -> u64 {
        let commit = handle.commit();
        let snapshot = self.snapshot();

        if snapshot.contains_key(&commit) {
            commit
        } else {
            let mut state = match self.sync().lock() {
                Ok(state) => state,
                Err(error) => error.into_inner(),
            };
            state.recorded.insert(commit, handle.clone());
            state.snapshot = LogSnapshot(Arc::new(state.recorded.clone()));
            commit
        }
    }

    /// Returns a reference to the inner map
    #[inline]
    fn snapshot(&self) -> LogSnapshot {
        let inner = match self.sync().lock() {
            Ok(state) => state,
            Err(err) => err.into_inner(),
        };
        inner.snapshot.clone()
    }

    /// Returns the sync context
    #[inline]
    fn sync(&self) -> &SyncContext {
        &self.sync
    }
}

impl Deref for LogSnapshot {
    type Target = BTreeMap<u64, Handle>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}

impl Resource for Journal {}

// #[test]
// fn test_journal() {
//     let journal = Journal::new();
//     let link = journal.log(
//         crate::ReprInternals::handle(&crate::repr::ty::TyRepr::new::<String>()),
//     );
//     assert!(link > 0);
//     assert!(journal.log.snapshot().keys().len() > 0);
// }
