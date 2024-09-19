use super::{EventConfig, PluginConfig};
use crate::{engine::env::EnvLoader, Result};
use reality::plugin::{Address, Event, HandlerThunk};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use tracing::debug;

/// Configures an engine environment
///
/// - `plugins`: map of plugins to load into the environment
/// - `handlers`: map of plugin handlers to load into the environment
///
/// The key of each map will be set as a label in Labels, `event = <key>`
///
/// ## Default file location
///
/// If a file location is not specified, this type will be constructed from the path format,
///
/// `<root>/<env>/config.toml`
#[derive(Default, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Plugins to be loaded by the environment
    #[serde(default)]
    plugins: BTreeMap<String, PluginConfig>,
    /// Plugin handlers to be loaded by the environment
    #[serde(default)]
    handlers: BTreeMap<String, PluginConfig>,
    /// Map of plugins loaded by this config
    #[serde(skip)]
    loaded_plugins: BTreeMap<String, Address>,
    /// Map of plugin handlers loaded by this config
    #[serde(skip)]
    loaded_handlers: BTreeMap<String, Address>,
}

impl Config {
    /// Tries to load an env engine config from some root directory, i.e. `<root>/<env>/config.toml`
    ///
    /// Returns an error if the file could not be read, found, or deserialized
    #[inline]
    pub fn from_file_system(root: impl Into<PathBuf>, name: &str) -> std::io::Result<Self> {
        let root = root.into();

        let config = root.join(name).join("config.toml");

        let config = std::fs::read_to_string(config)?;

        toml::from_str(&config)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.message()))
    }

    /// Load the engine config into state, creates map of loaded handlers and plugins
    #[inline]
    pub fn load(&mut self, loader: &mut EnvLoader) -> Result<()> {
        for (event_name, conf) in self.plugins.iter() {
            debug!("Loading event `{event_name}`");
            let address = conf.load(&event_name, loader)?;
            self.loaded_plugins.insert(event_name.to_string(), address);
        }

        for (handler_name, conf) in self.handlers.iter() {
            debug!("Loading handler `{handler_name}`");
            let address = conf.load(&handler_name, loader)?;
            self.loaded_handlers
                .insert(handler_name.to_string(), address);
        }
        Ok(())
    }

    /// Tries to return an event loaded by this config w/ the provided env loader
    ///
    /// Returns an error if the plugin could not found or event created
    #[inline]
    pub fn event(&self, name: &str, loader: &EnvLoader) -> reality::Result<Event> {
        self.loaded_plugins
            .get(name)
            .map(|p| loader.state.event(p))
            .unwrap_or(Err(reality::Error::PluginNotFound))
    }

    /// Tries to return a handler loaded by this config w/ the provided env loader
    ///
    /// Returns an error if the plugin could not found
    #[inline]
    pub fn handler(&self, name: &str, loader: &EnvLoader) -> reality::Result<HandlerThunk> {
        self.loaded_plugins
            .get(name)
            .map(|p| loader.state.handler(p))
            .unwrap_or(Err(reality::Error::PluginNotFound))
    }

    /// Tries to configure an event from an event config
    #[inline]
    pub fn config_event(&self, config: &EventConfig, loader: &EnvLoader) -> reality::Result<Event> {
        match config.split_for_lookup() {
            (event, None) => self.event(&event, loader),
            (event, Some(handler)) => {
                let mut event = self.event(&event, loader)?;
                let handler = self.handler(&handler, loader)?;
                event.set_handler(&handler)?;
                Ok(event)
            }
        }
    }
}
