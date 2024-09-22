use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;
use std::sync::Arc;

use runir::store::Item;
use tokio_util::sync::CancellationToken;

use super::ForkFn;
use super::Plugin;
use super::State;
use super::Work;
use crate::Error;
use crate::Result;

/// Contains state of a plugin invocation call
///
/// Serves as the context when a plugin is called
#[derive(Clone)]
pub struct Call {
    /// State which is the origin of this call
    pub(crate) state: State,
    /// Resource associated to this call
    pub(crate) item: Item,
    /// Function used to fork the item
    pub(crate) fork_fn: ForkFn,
    /// Child cancellation token that can be used to cancel this call
    pub(crate) cancel: CancellationToken,
    /// Handle to the backing runtime
    pub(crate) handle: tokio::runtime::Handle,
}

impl Call {
    /// Consumes this call context and binds a plugin to the current call,
    /// 
    /// Will receive any pending requests from state and call Plugin::receive in order to set the
    /// receiver binding on the plugin.
    ///
    /// Returns an error if the plugin does not match the current item in this context
    #[inline]
    #[must_use]
    pub fn bind<P: Plugin>(self) -> Result<Bind<P>> {
        if self.item.is_type::<P>() {
            let request = self.state.messages().receive(self.item.commit());
            let receiver = self
                .item
                .borrow::<P>()
                .and_then(|p| p.receive(request))
                .map(|p| p.into());
            Ok(Bind {
                call: self,
                receiver,
                _bound: PhantomData,
            })
        } else {
            Err(Error::PluginMismatch)
        }
    }

    /// Creates a fork of this call
    ///
    /// Will call `fork(item)` as well as create a child token for the inner cancel token
    #[inline]
    pub fn fork(&self) -> Call {
        Call {
            state: self.state.clone(),
            item: (self.fork_fn)(&self.item),
            fork_fn: self.fork_fn.clone(),
            cancel: self.cancel.child_token(),
            handle: self.handle.clone(),
        }
    }
}

/// Represents the binding between a plugin and it's associated Call
///
/// Main entrypoint for all plugins when they are invoked
pub struct Bind<P: Plugin> {
    /// Call this binding is associated to
    ///
    /// Before a binding is created, the association is verified
    pub(crate) call: Call,
    /// Receiver override for the plugin
    pub(crate) receiver: Option<Arc<P>>,
    /// Type this binding is bound to
    pub(crate) _bound: PhantomData<P>,
}

impl<P: Plugin> Bind<P> {
    /// Returns a reference to the plugin's "receiver"
    /// 
    /// A receiver is always immutable, so if the receiver field on the binding is set, that version of
    /// plugin state will be returned instead of the base item
    ///
    /// Returns an error if the current call context does not match the target plugin
    #[inline]
    pub fn receiver<'a: 'b, 'b>(&'a self) -> Result<&'b P> {
        match self.receiver.as_deref().or(self.call.item.borrow::<P>()) {
            Some(p) => Ok(p),
            None => Err(Error::PluginMismatch),
        }
    }

    /// Returns a mutable reference to the plugin in order to update the plugin's state
    ///
    /// Returns an error if the current call context does not match the target plugin
    #[inline]
    pub fn update(&mut self) -> Result<&mut P> {
        match self.call.item.borrow_mut::<P>() {
            Some(p) => Ok(p),
            None => Err(Error::PluginMismatch),
        }
    }

    /// Returns the current tokio handle
    #[inline]
    pub fn handle(&self) -> &tokio::runtime::Handle {
        &self.call.handle
    }

    /// Returns the item bound to this call
    #[inline]
    pub fn item(&self) -> &Item {
        &self.call.item
    }

    /// Defers access to the item for later by executing with the binding instead
    #[inline]
    pub fn defer<F>(
        self,
        exec: impl FnOnce(Bind<P>, CancellationToken) -> F + Send + 'static,
    ) -> Result<Work>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let binding = self.clone();
        let handle = self.call.handle.clone();
        let cancel_clone = self.call.cancel.clone();
        let cancel = self.call.cancel;
        Ok(Work {
            task: handle
                .clone()
                .spawn(async move { exec(binding, cancel_clone).await }),
            cancel,
        })
    }

    /// Consumes the call context and spawns returns work w/ mutable access to the plugin,
    ///
    /// Returns a join handle which will return work representing the running background task
    #[inline]
    pub fn work_mut<F>(
        self,
        exec: impl FnOnce(&mut P, CancellationToken) -> F + Send + 'static,
    ) -> Result<Work>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let mut call = self.clone();
        let handle = self.call.handle.clone();
        let cancel_clone = self.call.cancel.clone();
        let cancel = self.call.cancel;
        Ok(Work {
            task: handle
                .clone()
                .spawn(async move { exec(call.update()?, cancel_clone).await }),
            cancel,
        })
    }

    /// Consumes the call context and returns work w/ immutable access to the plugin,
    ///
    /// Returns a join handle which will return work representing the running background task
    #[inline]
    pub fn work<F>(
        self,
        exec: impl FnOnce(&P, CancellationToken) -> F + Send + 'static,
    ) -> Result<Work>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let call = self.clone();
        let handle = self.call.handle.clone();
        let cancel_clone = self.call.cancel.clone();
        let cancel = self.call.cancel;
        Ok(Work {
            task: handle
                .clone()
                .spawn(async move { exec(call.receiver()?, cancel_clone).await }),
            cancel,
        })
    }

    /// Convenience helper for calling returns `Err(Error::PluginCallSkipped)`
    #[inline]
    pub fn skip(self) -> crate::Result<Work> {
        Err(Error::PluginCallSkipped)
    }

    /// Convenience helper for constructing a plugin error
    #[inline]
    pub fn plugin_call_error(&self, message: impl Into<String>) -> crate::Error {
        crate::Error::PluginCallError {
            name: P::name(),
            message: message.into(),
        }
    }

    /// Convenience helper for returning a plugin call cancelled error
    #[inline]
    pub fn plugin_call_cancelled(&self) -> crate::Error {
        crate::Error::PluginCallCancelled
    }
}

impl<P: Plugin> Clone for Bind<P> {
    fn clone(&self) -> Self {
        Self {
            call: self.call.clone(),
            receiver: self.receiver.clone(),
            _bound: self._bound.clone(),
        }
    }
}

impl<P: Plugin> Debug for Bind<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bind")
            .field("_bound", &self._bound)
            .finish()
    }
}
