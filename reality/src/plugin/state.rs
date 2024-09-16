use super::Plugin;
use crate::Error;
use crate::{
    plugin::{Call, Thunk},
    Result,
};
use runir::store::Item;
use runir::{repo::Handle, repr::Attributes};
use std::future::Future;
use std::pin::Pin;
use std::{collections::BTreeMap, path::PathBuf};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Type-alias for a boxed future
type BoxFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

/// State contains manages registering and calling plugins
pub struct State {
    /// Store for resources owned by this state
    store: runir::Store,
    /// Cancellation token to stop any work related to this state
    cancel: CancellationToken,
    /// Handle to runtime to create work from state
    handle: tokio::runtime::Handle,
    /// Map of registered plugins
    plugins: BTreeMap<PathBuf, Handle>,
}

impl State {
    /// Returns a new state
    ///
    /// Panic: Can panic if not called within a tokio runtime
    #[inline]
    pub fn new() -> Self {
        Self {
            store: runir::Store::new(),
            cancel: CancellationToken::new(),
            handle: tokio::runtime::Handle::current(),
            plugins: BTreeMap::new(),
        }
    }

    /// Returns a new state w/ specified tokio runtime
    #[inline]
    pub fn with(handle: tokio::runtime::Handle) -> Self {
        Self {
            store: runir::Store::new(),
            cancel: CancellationToken::new(),
            handle,
            plugins: BTreeMap::new(),
        }
    }

    /// Initializes a new state
    ///
    /// **Note**: This call is safer since because when it is awaited, it will likely be inside of a tokio context
    #[inline]
    #[must_use]
    pub async fn init() -> Self {
        Self::new()
    }

    /// Registers a plugin w/ the the current state
    #[inline]
    pub fn register<P: Plugin>(&mut self, plugin: P) -> &mut Self {
        use crate::plugin::MustLoad;
        let name = P::name();
        let put = self.store.put(plugin);
        let handle = P::load(P::must_load(put)).commit();
        if let Some(_) = self.plugins.insert(name.path().clone(), handle) {
            // TODO: If a plugin was replaced, remove from the store
        }
        self
    }

    /// Calls a plugin, returns a future which can be awaited for the result
    ///
    /// Spawns the plugin immediately.
    ///
    /// ## Errors
    /// There are several error cases that can be returned
    ///
    /// - If the plugin is not registered
    /// - If the plugin does not return work
    /// - If access to the plugin could not be established
    /// - If the plugin did not return work
    #[inline]
    pub async fn call(&self, plugin: impl Into<PathBuf>) -> Result<()> {
        let (f, _) = self.spawn(plugin)?;
        f.await
    }

    /// Spawns the plugin
    /// 
    /// Returns the future and the associated cancellation token
    pub fn spawn(
        &self,
        plugin: impl Into<PathBuf>,
    ) -> Result<(BoxFuture, CancellationToken)> {
        let path = plugin.into();
        match self.plugins.get(&path).and_then(|h| {
            let id = h.commit();
            self.store
                .item(id)
                .zip(h.cast::<Attributes>().and_then(|a| a.get::<Thunk>()))
        }) {
            Some((item, thunk)) => {
                let plugin_name = thunk.name().to_string();
                debug!(name = plugin_name, "Preparing thunk");
                let cancel = self.cancel.child_token();
                let call = Call {
                    item: item.clone(),
                    cancel: cancel.clone(),
                    handle: self.handle.clone(),
                };

                Ok((
                    Box::pin(async move {
                        let work = thunk.exec(call).await?;
                        debug!(name = plugin_name, "Thunk binding complete, executing");
                        work.await
                    }),
                    cancel
                ))
            }
            None => Err(Error::CouldNotFindPlugin),
        }
    }

    /// Finds and returns a plugin item
    #[inline]
    pub fn find_plugin(&self, plugin: impl Into<PathBuf>) -> Option<&Item> {
        let path = plugin.into();
        self.plugins.get(&path).and_then(|h| {
            let id = h.commit();
            self.store.item(id)
        })
    }
}
