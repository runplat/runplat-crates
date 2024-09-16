mod thunk;
pub use thunk::Context;
pub use thunk::Thunk;

use std::hash::Hash;
use std::{future::Future, pin::pin};
use runir::{repr::Repr, Resource};
use tokio_util::sync::CancellationToken;

use crate::Error;

/// Plugin trait for implementing extensions within the reality framework
pub trait Plugin {
    /// Namespace of this plugin
    fn namespace() -> Namespace;

    /// Invoked when activating the plugin
    /// 
    /// Returns an async context if the plugin should activate
    fn call(context: Context) -> Option<AsyncContext>;

    /// Invoked when this plugin is loaded into a context
    fn load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self>
    where 
        Self: Resource + Hash + Sized
    {
        put.attr(Thunk::new::<Self>())
    }
}

/// Struct containing name data
#[derive(Hash, Clone, Default)]
pub struct Namespace {
    /// Root of this namespace
    root: String,
}

impl Repr for Namespace {}
impl Resource for Namespace {}

pub struct AsyncContext {
    task: tokio::task::JoinHandle<crate::Result<Context>>,
    cancel: CancellationToken
}

impl Future for AsyncContext {
    type Output = crate::Result<Context>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.cancel.is_cancelled() {
            self.task.abort();
            return std::task::Poll::Ready(Err(Error::PluginAborted));
        }
        let task = self.as_mut();
        let pinned = pin!(task);
        match pinned.poll(cx) {
            std::task::Poll::Ready(r) => {
                std::task::Poll::Ready(Ok(r?))
            },
            std::task::Poll::Pending => {
                std::task::Poll::Pending
            },
        }
    }
}