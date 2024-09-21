use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::plugin::LoadSource;

pub trait Metadata {
    fn build(&self) -> Option<&Build> {
        None
    }
    
    fn loader(&self) -> Option<&Loader> {
        None
    }
}

/// Build metadata that can be used to build a collection of .toml files
/// Can be deserialized by plugins with the field name "-kt-build"
#[derive(Debug, Serialize, Deserialize)]
pub struct Build {
    /// Name of the plugin
    pub plugin: String,
    /// Load source setting
    pub load: Option<LoadSource>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// True if the plugin should be added as a handler
    pub handler: Option<BuildHandler>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildHandler {
    target: Option<String>
}

/// Loader metadata that can be used to build a collection of .toml files
/// Can be deserialized by plugins with the field name "-kt-load"
#[derive(Debug, Serialize, Deserialize)]
pub struct Loader {
    /// Name of the environment that loaded this plugin
    pub env: String,
    /// Root directory of the environment that loaded this plugin
    pub root: PathBuf,
    /// Path to source file that loaded this Loader
    pub src: PathBuf,
    /// File size of the source file that loaded this loader
    #[serde(rename = "src-size")]
    pub src_size: u64,
    /// Event or Event Handler identifier that was loader
    pub event: String,
    /// CRC digest of the source using the CRC_64_MS algo
    #[serde(rename = "crc-ms")]
    pub crc_ms: String
}

impl Loader {
    pub fn split_for_env_loader(&self) -> (String, std::io::Result<PathBuf>) {
        (self.env.to_string(), Ok(self.root.clone()))
    }
}