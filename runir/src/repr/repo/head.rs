use super::*;

/// Head representation for managing related repo commits
pub struct Head<R> {
    /// Commit ID assigned by the repo when the head representation was created
    commit: u64,
    /// Inner Data
    inner: Pin<Arc<R>>,
}

impl<R: Repr> Head<R> {
    /// Creates a new Head representation
    #[inline]
    pub fn new(commit: u64, repr: R) -> Self {
        Self {
            commit,
            inner: Arc::pin(repr),
        }
    }

    /// Returns a handle
    #[inline]
    #[must_use]
    pub fn handle(&self) -> Handle {
        Handle::new::<R>(self.commit, self.inner.clone())
    }
}

impl<R> Clone for Head<R> {
    fn clone(&self) -> Self {
        Self {
            commit: self.commit,
            inner: self.inner.clone(),
        }
    }
}
