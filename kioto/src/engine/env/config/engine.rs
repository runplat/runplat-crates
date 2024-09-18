use std::{collections::BTreeMap, path::PathBuf};

use crate::Result;
use reality::State;
use serde::{Deserialize, Serialize};

use super::PluginConfig;

/// Configures an engine environment
/// 
/// - `plugins`: map of plugins to load into the environment
/// - `handlers`: map of plugin handlers to load into the environment
/// 
/// The key of each map will be set as a label in Labels, `event = <key>`
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Plugins to be loaded by the environment
    #[serde(default)]
    plugins: BTreeMap<String, PluginConfig>,
    /// Plugin handlers to be loaded by the environment
    #[serde(default)]
    handlers: BTreeMap<String, PluginConfig>,
}

impl Config {
    /// Load the engine config into state
    pub fn load(&self, state: &mut State, root: PathBuf) -> Result<()> {
        Ok(())
    }
}