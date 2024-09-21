use std::{collections::BTreeMap, path::PathBuf};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use super::{plugin::LoadSource, TemplateMap};

pub trait Metadata {
    fn build(&self) -> Option<&Build> {
        None
    }

    fn loader(&self) -> Option<&Loader> {
        None
    }

    /// Apply template configuration from build metadata and return updated state
    fn apply_template_toml_data(&self, data: &toml::Table) -> std::io::Result<Self>
    where
        Self: Serialize + DeserializeOwned
    {
        match self.build().and_then(|b| b.templates.as_ref()) {
            Some(templates) => {
                let map = TemplateMap::from(templates);
                map.apply_toml(self, data)
            },
            None => {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No template fields declared in build metadata"))
            },
        }
    }

    /// Apply template configuration from build metadata and return updated state
    fn apply_template_json_data(&self, data: &serde_json::Map<String, serde_json::Value>) -> std::io::Result<Self>
    where
        Self: Serialize + DeserializeOwned
    {
        match self.build().and_then(|b| b.templates.as_ref()) {
            Some(templates) => {
                let map = TemplateMap::from(templates);
                map.apply_json(self, data)
            },
            None => {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "No template fields declared in build metadata"))
            },
        }
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
    /// Map of labels to include when loading the plugin into state
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    /// Map of fields that are template strings and a config of the
    /// expected input values
    /// 
    /// # Example Usage
    /// ```toml
    /// # Indicates that the url field below is using a template string
    /// # Each field must be declared
    /// -kt-build.templates.url.host = ""
    /// # The field can also be an inline table w/ various settings
    /// -kt-build.templates.url.path = { match = "<regex>", default = "/posts",  }
    /// 
    /// url = "https://{{host}}/{{path}}"
    /// ```
    pub templates: Option<BTreeMap<String, toml::Table>>,
    /// True if the plugin should be added as a handler
    pub handler: Option<BuildHandler>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildHandler {
    target: Option<String>,
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
    pub crc_ms: String,
}

impl Loader {
    pub fn split_for_env_loader(&self) -> (String, std::io::Result<PathBuf>) {
        (self.env.to_string(), Ok(self.root.clone()))
    }
}
