use super::Plugin;
use crate::Error;
use crate::{
    plugin::{Call, Thunk},
    Result,
};
use runir::store::Item;
use runir::{repo::Handle, repr::Attributes};
use std::{collections::BTreeMap, path::PathBuf};
use tokio_util::sync::CancellationToken;

pub struct State {
    /// Store for resources owned by this state
    store: runir::Store,
    /// Cancellation token to stop any work related to this state
    cancel: CancellationToken,
    /// Handle to runtime to create work from state
    handle: tokio::runtime::Handle,
    /// Map of registered plugins
    ///
    /// A plugin can only be registered once
    plugins: BTreeMap<PathBuf, Handle>,
}

impl State {
    /// Returns a new state
    ///
    /// Panic: Can panic if not called within a tokio runtime
    pub fn new() -> Self {
        Self {
            store: runir::Store::new(),
            cancel: CancellationToken::new(),
            handle: tokio::runtime::Handle::current(),
            plugins: BTreeMap::new(),
        }
    }

    pub fn with(handle: tokio::runtime::Handle) -> Self {
        Self {
            store: runir::Store::new(),
            cancel: CancellationToken::new(),
            handle,
            plugins: BTreeMap::new(),
        }
    }

    /// Initializes a new state inside of a tokio runtime
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

    /// Calls a plugin, returns a future which can be awaited for the entire process
    ///
    /// ## Errors
    /// There are several error cases that can be returned
    ///
    /// - If the plugin is not registered
    /// - If the plugin does not return work
    /// - If access to the plugin could not be established
    /// - If the plugin did not return work
    #[must_use = "If the future is not awaited, then the call cannot be executed"]
    pub async fn call(&self, plugin: impl Into<PathBuf>) -> Result<()> {
        let path = plugin.into();
        match self.plugins.get(&path).and_then(|h| {
            let id = h.commit();
            self.store
                .item(id)
                .zip(h.cast::<Attributes>().and_then(|a| a.get::<Thunk>()))
        }) {
            Some((item, thunk)) => {
                let call = Call {
                    item: item.clone(),
                    cancel: self.cancel.child_token(),
                    handle: self.handle.clone(),
                };

                let work = thunk.exec(call).await;
                work?.await
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
