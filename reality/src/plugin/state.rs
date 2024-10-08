use super::{thunk::HandlerThunk, Address, Broker, Handler, Name, Plugin};
use crate::{
    plugin::{event::Event, Call, Thunk},
    Error, Result,
};
use clap::ArgMatches;
use runir::{
    repo::Handle,
    repr::{Attributes, Labels},
    store::Item,
    Store,
};
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    future::Future,
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, RwLock},
};
use tokio_util::sync::CancellationToken;
use tracing::debug;

/// Type-alias for a boxed future
type BoxFuture = Pin<Box<dyn Future<Output = Result<()>>>>;

type PluginMap = std::sync::Arc<std::sync::RwLock<BTreeMap<PathBuf, Handle>>>;

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
    plugins: PluginMap,
    /// Message system
    messages: Broker,
    /// If set to true, will return an error if a plugin being loaded
    /// will overwrite an existing plugin
    disallow_commit_conflicts: bool,
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
            plugins: Arc::new(RwLock::new(BTreeMap::new())),
            messages: Broker::default(),
            disallow_commit_conflicts: false,
        }
    }

    /// Returns a new state w/ specified tokio runtime
    #[inline]
    pub fn with(handle: tokio::runtime::Handle) -> Self {
        Self {
            store: runir::Store::new(),
            cancel: CancellationToken::new(),
            handle,
            plugins: Arc::new(RwLock::new(BTreeMap::new())),
            messages: Broker::default(),
            disallow_commit_conflicts: false,
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

    /// If set to true, will return an error if a plugin being loaded would overwrite an existing plugin commit
    #[inline]
    pub fn disallow_commit_conflicts(&mut self, disallow: bool) {
        self.disallow_commit_conflicts = disallow;
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

    /// Returns a reference to messagge state
    #[inline]
    pub fn broker(&self) -> &Broker {
        &self.messages
    }

    /// Registers a plugin from parsing cli arg matches
    #[inline]
    pub fn load_by_args<P: Plugin + clap::FromArgMatches>(
        &mut self,
        matches: &ArgMatches,
        labels: Labels,
    ) -> std::io::Result<Address> {
        let plugin = P::from_arg_matches(matches)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        Ok(self.load(plugin, labels))
    }

    /// Loads and registers a plugin from toml
    #[inline]
    pub fn load_by_toml<P: Plugin + DeserializeOwned>(
        &mut self,
        toml: &str,
        labels: Labels,
    ) -> std::io::Result<Address> {
        let plugin = toml::from_str::<P>(toml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.message()))?;
        Ok(self.load(plugin, labels))
    }

    /// Registers a plugin w/ the the current state
    #[inline]
    pub fn load<P: Plugin>(&mut self, plugin: P, labels: Labels) -> Address {
        use crate::plugin::MustLoad;
        let name = P::name();

        // TODO: Might want to refactor this to return a Load builder
        let mut put = self.store.put(plugin);
        for (k, v) in labels.iter() {
            put = put.label(k, v);
        }
        let handle = P::load(P::must_load(put)).commit();
        let address = name.path().join(hex::encode(handle.commit().to_be_bytes()));

        let mut plugins = match self.plugins.write() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };

        plugins.insert(name.path().clone(), handle.clone());
        if let Some(_) = plugins
            .insert(address, handle.clone())
            .filter(|_| self.disallow_commit_conflicts)
        {
            todo!("Commit conflicts disallowed")
        }

        Address {
            name,
            commit: handle.commit(),
        }
    }

    /// Registers a plugin from parsing cli arg matches
    #[inline]
    pub fn load_handler_by_args<H: Handler + clap::FromArgMatches>(
        &mut self,
        matches: &ArgMatches,
        labels: Labels,
    ) -> std::io::Result<Address> {
        let plugin = H::from_arg_matches(matches)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;

        Ok(self.load_handler(plugin, labels))
    }

    /// Loads and registers a plugin from toml
    #[inline]
    pub fn load_handler_by_toml<H: Handler + DeserializeOwned>(
        &mut self,
        toml: &str,
        labels: Labels,
    ) -> std::io::Result<Address> {
        let plugin = toml::from_str::<H>(toml)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.message()))?;
        Ok(self.load_handler(plugin, labels))
    }

    /// Registers a plugin w/ the the current state
    #[inline]
    pub fn load_handler<H: Handler>(&mut self, plugin: H, labels: Labels) -> Address {
        use crate::plugin::MustLoadHandler;
        let name = H::name();
        let mut put = self.store.put(plugin);
        for (k, v) in labels.iter() {
            put = put.label(k, v);
        }
        let handle = H::load(H::must_load(put)).commit();
        let mut plugins = match self.plugins.write() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        plugins.insert(name.path().clone(), handle.clone());
        plugins.insert(
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
        let plugins = match self.plugins.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        match plugins.get(&path).and_then(|h| {
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
                    runtime: self.handle.clone(),
                    handler: None,
                };
                let labels = item.attributes().get::<Labels>();

                Ok(Event {
                    address,
                    call,
                    thunk: thunk.as_ref().clone(),
                    handler: None,
                    labels,
                })
            }
            None => Err(Error::PluginNotFound),
        }
    }

    /// Find and returns a handler thunk from a plugin path
    #[inline]
    pub fn handler(&self, plugin: impl Into<PathBuf>) -> Result<HandlerThunk> {
        let path = plugin.into();
        let plugins = match self.plugins.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        match plugins
            .get(&path)
            .and_then(|h| h.cast::<Attributes>().and_then(|a| a.get::<HandlerThunk>()))
        {
            Some(h) => Ok(h.deref().clone()),
            None => Err(Error::PluginNotFound),
        }
    }

    /// Finds and returns a plugin item
    #[inline]
    pub fn find_plugin(&self, plugin: impl Into<PathBuf>) -> Option<&Item> {
        let path = plugin.into();
        let plugins = match self.plugins.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        plugins.get(&path).and_then(|h| {
            let id = h.commit();
            self.store.item(id)
        })
    }

    /// Returns each unique address stored in state
    #[inline]
    pub fn addresses(&self) -> Vec<Address> {
        let plugins = match self.plugins.read() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        plugins
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
            .collect::<Vec<Address>>()
    }
}
