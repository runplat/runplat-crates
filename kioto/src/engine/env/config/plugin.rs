use crate::{engine::env::EnvLoader, Errors, PluginLoadErrors, Result};
use reality::plugin::{Address, Name};
use serde::{Deserialize, Serialize};
use tracing::debug;
use std::{collections::BTreeMap, io::Read, path::PathBuf, str::FromStr};

/// Define settings settings for configuring a plugin
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Config {
    /// Plugin being loaded
    plugin: String,
    /// Plugin load settings
    load: Option<LoadSource>,
    /// Labels to add as an attribute after loading the plugin
    #[serde(default)]
    labels: BTreeMap<String, String>,
}

impl Config {
    /// Loads the plugin from this config into state
    ///
    /// Returns an error if the plugin could not be loaded successfully
    #[inline]
    pub fn load(&self, event: &str, loader: &mut EnvLoader) -> Result<Address> {
        let name = Name::from_str(&self.plugin)?;
        if let Some(load) = self.load.as_ref() {
            match load {
                LoadSource::File {
                    path,
                    format: SourceFormats::Toml,
                } => load_toml(event, name, path, loader),
            }
        } else {
            let path = loader
                .root_dir
                .join("etc")
                .join(name.path())
                .join(format!("{event}.toml"));
            load_toml(event, name, &path, loader)
        }
    }
}

/// Loads toml from an env loader
fn load_toml(event: &str, name: Name, path: &PathBuf, loader: &mut EnvLoader) -> Result<Address> {
    debug!("Trying to load {path:?}");
    match std::fs::OpenOptions::new().read(true).open(path) {
        Ok(mut opened) => {
            let mut toml = String::new();
            match opened.read_to_string(&mut toml) {
                Ok(_) => {
                    let settings = toml_edit::DocumentMut::from_str(&toml).unwrap();
                    Ok(loader.load(&name, settings.as_table()).unwrap())
                }
                Err(io) => Err(Errors::PluginLoadError(
                    PluginLoadErrors::CouldNotReadFile {
                        error: crate::CouldNotLoadPlugin::new(event, name),
                        io,
                    },
                )),
            }
        }
        Err(io) => Err(Errors::PluginLoadError(
            PluginLoadErrors::CouldNotReadFile {
                error: crate::CouldNotLoadPlugin::new(event, name),
                io,
            },
        )),
    }
}

/// Enumeration of load plugin source variants
#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(tag = "type")]
pub enum LoadSource {
    /// Load source is from a file path
    #[serde(rename = "file")]
    File {
        /// Path of the source to load, the last component should be the name of the file, and the
        /// four components before that should be the path of the plugin.
        ///
        /// For example, a plugin named "login" that defines a `kioto/plugins.request` plugin would be by default stored
        /// at `kioto/0.1.0/plugins/request/login.toml`.
        #[serde(default)]
        path: PathBuf,
        /// Format this source is in
        #[serde(default)]
        format: SourceFormats,
    },
}

/// Enumeration of supported source formats
#[derive(Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum SourceFormats {
    /// File is a valid toml
    #[serde(rename = "toml")]
    #[default]
    Toml,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::Request;
    use reality::Plugin;

    #[test]
    fn test_deser_config_enum_types() {
        let s = toml::from_str::<Config>(
            r#"
plugin = "kioto/plugins.request"
load = { type = "file", path = "etc/test", format = "toml" }
"#,
        )
        .unwrap();
        assert_eq!(
            Config {
                plugin: Request::name().to_string(),
                load: Some(LoadSource::File {
                    path: PathBuf::from("etc/test"),
                    format: SourceFormats::Toml
                }),
                labels: BTreeMap::new()
            },
            s
        );
    }

    #[test]
    fn test_deser_config_defaults() {
        let s = toml::from_str::<Config>(
            r#"
plugin = "kioto/plugins.request"
load = { type = "file" }
"#,
        )
        .unwrap();
        assert_eq!(
            Config {
                plugin: Request::name().to_string(),
                load: Some(LoadSource::File {
                    path: PathBuf::default(),
                    format: SourceFormats::Toml
                }),
                labels: BTreeMap::new()
            },
            s
        );
    }
}
