use super::{Address, Name, Plugin};
use crate::plugin::event::Event;
use crate::Error;
use crate::{
    plugin::{Call, Thunk},
    Result,
};
use clap::ArgMatches;
use runir::store::{Item, Put};
use runir::Store;
use runir::{repo::Handle, repr::Attributes};
use serde::de::DeserializeOwned;
use std::future::Future;
use std::ops::Deref;
use std::pin::Pin;
use std::{collections::BTreeMap, path::PathBuf};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Type-alias for a boxed future
type BoxFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

/// State contains manages registering and calling plugins
#[derive(Clone)]
pub struct State {
    /// Store for resources owned by this state
    store: runir::Store,
    /// Cancellation token to stop any work related to this state
    pub(crate) cancel: CancellationToken,
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

    /// Returns a reference to the inner store
    #[inline]
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Returns a mutable reference to the inner store
    #[inline]
    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    /// Closes this state by cancelling the inner cancel token
    #[inline]
    pub fn close(&self) {
        self.cancel.cancel()
    }

    /// Registers a plugin from parsing cli arg matches
    #[inline]
    pub fn load_by_args<P: Plugin + clap::FromArgMatches>(
        &mut self,
        matches: &ArgMatches,
    ) -> std::io::Result<Address> {
        self.load_by_args_with(matches, |p: Put<'_, P>| p)
    }

    /// Loads and registers a plugin from toml
    #[inline]
    pub fn load_by_toml<P: Plugin + DeserializeOwned>(
        &mut self,
        toml: &str,
    ) -> std::io::Result<Address> {
        self.load_by_toml_with(toml, |p: Put<'_, P>| p)
    }

    /// Registers a plugin w/ the the current state
    #[inline]
    pub fn load<P: Plugin>(&mut self, plugin: P) -> Address {
        self.load_with(plugin, |p: Put<'_, P>| p)
    }

    /// Registers a plugin from parsing cli arg matches
    #[inline]
    pub fn load_by_args_with<P: Plugin + clap::FromArgMatches>(
        &mut self,
        matches: &ArgMatches,
        extend: impl Fn(runir::store::Put<'_, P>) -> runir::store::Put<'_, P>,
    ) -> std::io::Result<Address> {
        let plugin = P::from_arg_matches(matches)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        Ok(self.load_with(plugin, extend))
    }

    /// Loads and registers a plugin from toml
    #[inline]
    pub fn load_by_toml_with<P: Plugin + DeserializeOwned>(
        &mut self,
        toml: &str,
        extend: impl Fn(runir::store::Put<'_, P>) -> runir::store::Put<'_, P>,
    ) -> std::io::Result<Address> {
        let plugin = toml::from_str::<P>(toml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.message()))?;
        Ok(self.load_with(plugin, extend))
    }

    /// Registers a plugin w/ the the current state
    #[inline]
    pub fn load_with<P: Plugin>(
        &mut self,
        plugin: P,
        extend: impl Fn(runir::store::Put<'_, P>) -> runir::store::Put<'_, P>,
    ) -> Address {
        use crate::plugin::MustLoad;
        let name = P::name();
        let put = self.store.put(plugin);
        let handle = extend(P::load(P::must_load(put))).commit();
        self.plugins.insert(name.path().clone(), handle.clone());
        self.plugins.insert(
            name.path().join(hex::encode(handle.commit().to_be_bytes())),
            handle.clone(),
        );

        Address {
            name,
            commit: handle.commit(),
        }
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

    /// Spawns a call to a plugin
    ///
    /// Returns the future and the associated cancellation token
    #[inline]
    pub fn spawn(&self, plugin: impl Into<PathBuf>) -> Result<(BoxFuture, CancellationToken)> {
        let event = self.event(plugin)?;
        let (f, cancel) = event.fork();
        Ok((Box::pin(f.start()), cancel))
    }

    /// Creates a new "Event" for a plugin
    pub fn event(&self, plugin: impl Into<PathBuf>) -> Result<Event> {
        let path = plugin.into();
        match self.plugins.get(&path).and_then(|h| {
            let id = h.commit();
            self.store
                .item(id)
                .zip(h.cast::<Attributes>().and_then(|a| a.get::<Thunk>()))
                .map(|(i, t)| {
                    (
                        Address {
                            name: t.name().clone(),
                            commit: id,
                        },
                        i,
                        t,
                    )
                })
        }) {
            Some((address, item, thunk)) => {
                let plugin_name = thunk.name().to_string();
                debug!(name = plugin_name, "Preparing thunk");
                let cancel = self.cancel.child_token();
                let call = Call {
                    state: self.clone(),
                    item: item.clone(),
                    fork_fn: thunk.fork_fn(),
                    cancel: cancel.clone(),
                    handle: self.handle.clone(),
                };

                Ok(Event {
                    address,
                    call,
                    thunk: thunk.as_ref().clone(),
                    handler: None,
                })
            }
            None => Err(Error::PluginNotFound),
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

    /// Returns each unique address stored in state
    #[inline]
    pub fn addresses(&self) -> impl Iterator<Item = Address> + '_ {
        self.plugins
            .iter()
            .filter(|(p, _)| p.ancestors().count() > 5)
            .filter_map(|(_, h)| {
                let id = h.commit();
                self.store
                    .item(id)
                    .and_then(|i| i.attributes().get::<Name>())
                    .zip(Some(id))
            })
            .map(|(name, id)| Address {
                name: name.deref().clone(),
                commit: id,
            })
    }
}
