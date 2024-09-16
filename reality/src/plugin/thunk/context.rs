use std::{future::Future, hash::Hash};

use runir::{Resource, Store};
use tokio_util::sync::CancellationToken;

use crate::plugin::{AsyncContext, Plugin};

use super::Thunk;

#[derive(Clone)]
pub struct Context {
    /// runir store
    store: runir::Store,
    cancellation: CancellationToken,
    handle: tokio::runtime::Handle,
}

impl Context {
    /// Loads a plugin into the current context
    #[inline]
    pub fn load<P: Plugin + Hash + Resource>(&mut self, resource: P) {
        let put = self.store.put(resource);
        let _ = P::load(put).commit();
    }

    /// Spawns a future w/ the current context
    #[inline]
    pub fn spawn<F>(self, spawn: impl FnOnce(CancellationToken) -> F + Send + 'static) -> AsyncContext
    where
        F: Future<Output = crate::Result<Context>> + Send + 'static,
    {
        let cancel = self.cancellation.child_token();
        let _ct = cancel.clone();
        AsyncContext {
            task: self.handle.spawn(async move { spawn(_ct).await }),
            cancel,
        }
    }

    /// Returns a reference to the store
    #[inline]
    pub fn store(&self) -> &Store {
        &self.store
    }
}
