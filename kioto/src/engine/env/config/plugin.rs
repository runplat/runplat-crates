use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};

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
    use reality::Plugin;
    use crate::plugins::Request;
    use super::*;

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
