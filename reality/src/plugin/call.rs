use std::fmt::Debug;
use std::future::Future;
use std::marker::PhantomData;

use runir::store::Item;
use tokio_util::sync::CancellationToken;

use super::Plugin;
use super::Work;
use crate::Error;
use crate::Result;

/// Contains state of a plugin invocation call
///
/// Serves as the context when a plugin is called
#[derive(Clone)]
pub struct Call {
    /// Resource associated to this call
    pub(crate) item: Item,
    /// Child cancellation token that can be used to cancel this call
    pub(crate) cancel: CancellationToken,
    /// Handle to the backing runtime
    pub(crate) handle: tokio::runtime::Handle,
}

impl Call {
    /// Consumes this call context and binds a plugin to the current call
    ///
    /// Returns an error if the plugin does not match the current item in this context
    #[inline]
    #[must_use]
    pub fn bind<P: Plugin>(self) -> Result<Bind<P>> {
        if self.item.is_type::<P>() {
            Ok(Bind {
                call: self,
                _bound: PhantomData,
            })
        } else {
            Err(Error::PluginMismatch)
        }
    }
}

pub struct Bind<P: Plugin> {
    pub(crate) call: Call,
    pub(crate) _bound: PhantomData<P>,
}

impl<P: Plugin> Bind<P> {
    /// Returns a reference to the plugin's resource
    ///
    /// Returns an error if the current call context does not match the target plugin
    #[inline]
    pub fn plugin<'a: 'b, 'b>(&'a self) -> Result<&'b P> {
        match self.call.item.borrow::<P>() {
            Some(p) => Ok(p),
            None => Err(Error::PluginMismatch),
        }
    }

    /// Returns a mutable reference to the plugin's resource
    ///
    /// Returns an error if the current call context does not match the target plugin
    #[inline]
    pub fn plugin_mut(&mut self) -> Result<&mut P> {
        match self.call.item.borrow_mut::<P>() {
            Some(p) => Ok(p),
            None => Err(Error::PluginMismatch),
        }
    }

    /// Consumes the call context and spawns returns work w/ mutable access to the plugin,
    ///
    /// Returns a join handle which will return work representing the running background task
    #[inline]
    pub fn work_mut<F>(
        mut self,
        exec: impl FnOnce(&mut P, CancellationToken) -> F + Send + 'static,
    ) -> tokio::task::JoinHandle<Result<Work>>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let handle = self.call.handle.clone();
        let cancel = self.call.cancel.clone();

        // Since getting access to the plugin can block, execute on the blocking thread pool
        handle.clone().spawn_blocking(move || {
            let plugin = self.plugin_mut()?;
            Ok(Work {
                task: handle.spawn(exec(plugin, cancel.clone())),
                cancel,
            })
        })
    }

    /// Consumes the call context and returns work w/ immutable access to the plugin,
    ///
    /// Returns a join handle which will return work representing the running background task
    #[inline]
    pub fn work<F>(
        self,
        exec: impl FnOnce(&P, CancellationToken) -> F + Send + 'static,
    ) -> tokio::task::JoinHandle<Result<Work>>
    where
        F: Future<Output = Result<()>> + Send + 'static,
    {
        let handle = self.call.handle.clone();
        let cancel = self.call.cancel.clone();
        // Since getting access to the plugin can block, execute on the blocking thread pool
        handle.clone().spawn_blocking(move || {
            let call = self.clone();
            let task = exec(call.plugin()?, cancel.clone());
            Ok(Work {
                task: handle.spawn(task),
                cancel,
            })
        })
    }
}

impl<P: Plugin> Clone for Bind<P> {
    fn clone(&self) -> Self {
        Self {
            call: self.call.clone(),
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
