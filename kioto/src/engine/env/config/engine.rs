use super::{EventConfig, PluginConfig};
use crate::{engine::env::Env, Result};
use reality::plugin::{Address, Event, HandlerThunk, Name};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use toml_edit::DocumentMut;
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
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
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
    /// Tries to parse build metadata from a document,
    ///
    /// Returns an error message if unsuccessful, otherwise adds the plugin config to the current config
    pub fn parse_build_document(
        &mut self,
        event_name: impl Into<String>,
        doc: DocumentMut,
    ) -> std::result::Result<Name, String> {
        let event_name = event_name.into();
        if let Some(table) = doc.get("-kt-build").and_then(|t| t.as_table_like()) {
            let table: toml_edit::Table = table.iter().collect();
            match toml::from_str::<super::BuildMetadata>(&format!("{}", table)) {
                Ok(metadata) => {
                    match metadata.plugin.parse::<Name>() {
                        Ok(name) => {
                            // debug!("Building file {:?}", entry.path());
                            if let Some(_) = metadata.handler.as_ref() {
                                self.handlers.insert(
                                    event_name,
                                    PluginConfig {
                                        plugin: metadata.plugin.to_string(),
                                        load: metadata.load.clone(),
                                        labels: metadata.labels.clone(),
                                    },
                                );
                            } else {
                                self.plugins.insert(
                                    event_name,
                                    PluginConfig {
                                        plugin: metadata.plugin.to_string(),
                                        load: metadata.load.clone(),
                                        labels: metadata.labels.clone(),
                                    },
                                );
                            }
                            Ok(name)
                        }
                        Err(err) => Err(format!("Could not parse declared plugin {err:?}")),
                    }
                }
                Err(err) => Err(format!("Parsing `-kt-build` failed: {}", err.message())),
            }
        } else {
            Err(format!("Skipping toml file `-kt-build` table not found"))
        }
    }

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
    pub fn load(&mut self, loader: &mut Env) -> Result<()> {
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
    pub fn event(&self, name: &str, loader: &Env) -> reality::Result<Event> {
        self.loaded_plugins
            .get(name)
            .map(|p| loader.state.event(p))
            .unwrap_or(Err(reality::Error::PluginNotFound))
    }

    /// Tries to return a handler loaded by this config w/ the provided env loader
    ///
    /// Returns an error if the plugin could not found
    #[inline]
    pub fn handler(&self, name: &str, loader: &Env) -> reality::Result<HandlerThunk> {
        self.loaded_handlers
            .get(name)
            .map(|p| loader.state.handler(p))
            .unwrap_or(Err(reality::Error::PluginNotFound))
    }

    /// Tries to configure an event from loaded plugins and handlers
    ///
    /// If the event config specifies a handler, the handler will be applied to the returned event
    #[inline]
    pub fn configure_event(&self, config: &EventConfig, loader: &Env) -> reality::Result<Event> {
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
