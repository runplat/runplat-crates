mod config;
pub use config::EngineConfig;
pub use config::EventConfig;


use std::{collections::BTreeSet, path::PathBuf};
use clap::FromArgMatches;
use serde::de::DeserializeOwned;
use super::{Load, LoadInput};
use crate::plugins::{Request, RequestArgs};
use reality::{plugin::Name, repo::Handle, Content, Plugin, Repr, Resource, State, Uuid};

/// Type-alias for a function that creates an environment
type CreateLoader = fn() -> EnvLoader;

/// Struct containing tools for creating a new environment
pub struct Env {
    /// Label for this environment
    label: String,
    /// Engine config for this environment
    config: Option<EngineConfig>,
    /// Function for creating a new environment
    create_loader: CreateLoader,
}

impl Env {
    /// Creates a new env repr
    #[inline]
    pub fn new(label: impl Into<String>, crate_loader: CreateLoader) -> Self {
        Self {
            label: label.into(),
            config: None,
            create_loader: crate_loader,
        }
    }

    /// Sets the config for the environment
    #[inline]
    pub fn with_config(&mut self, config: EngineConfig) {
        self.config = Some(config);
    }

    /// Creates a new env loader
    #[inline]
    pub fn loader(&self) -> EnvLoader {
        (self.create_loader)()
    }
}

impl Resource for Env {}
impl Repr for Env {}
impl Content for Env {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        let mut crc = reality::content::crc().digest();
        crc.update(self.label.as_bytes());
        Uuid::from_u64_pair(crc.finalize(), 0)
    }
}

impl Default for Env {
    fn default() -> Self {
        Self {
            label: String::from("default"),
            config: None,
            create_loader: default_env,
        }
    }
}

/// Struct containing state for loading an environment
pub struct EnvLoader {
    pub root_dir: PathBuf,
    pub state: State,
    pub loaders: BTreeSet<(Name, Handle)>,
}

impl EnvLoader {
    /// Adds a toml loader to the env loader
    #[inline]
    pub fn add_toml_loader<P: Plugin + DeserializeOwned>(&mut self) {
        let h = self.state.store_mut().put(Load::by_toml::<P>()).commit();
        self.loaders.insert((P::name(), h));
    }

    /// Adds an arg loader to the env loader
    #[inline]
    pub fn add_args_loader<P: Plugin + FromArgMatches>(&mut self) {
        let h = self.state.store_mut().put(Load::by_args::<P>()).commit();
        self.loaders.insert((P::name(), h));
    }

    /// Finds a loader by name
    #[inline]
    pub fn find_loader(&self, name: &Name) -> Option<Load> {
        self.loaders
            .iter()
            .find(|(n, _)| n.full_plugin_ref() == name.full_plugin_ref())
            .or(self
                .loaders
                .iter()
                .find(|(n, _)| n.plugin_ref() == name.plugin_ref()))
            .and_then(|(_, l)| self.state.store().item(l.commit()))
            .and_then(|i| i.borrow::<Load>().cloned())
    }

    /// Tries to load a plugin w/ input
    ///
    /// Returns an error if the loader could not be found, or if the loader returns an error
    #[inline]
    pub fn load(
        &mut self,
        name: &Name,
        input: impl Into<LoadInput>,
    ) -> Result<reality::plugin::Address, std::io::Error> {
        match self.find_loader(name) {
            Some(load) => load.load(&mut self.state, input),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find loader for plugin",
            )),
        }
    }
}

/// Creates an env w/ default set of plugins
pub fn default_env() -> EnvLoader {
    let mut loader = EnvLoader {
        root_dir: PathBuf::from(".kt").join("default"),
        state: State::new(),
        loaders: BTreeSet::new(),
    };
    loader.add_toml_loader::<Request>();
    loader.add_args_loader::<RequestArgs>();
    loader
}

/// Creates a new test Env
#[macro_export]
macro_rules! test_env {
    ($vis:vis $name:ident) => {
        $vis mod $name {
            use crate::engine::*;

            /// Creates a new env
            pub fn loader() -> Env {
                Env {
                    label: String::from(stringify!($name)),
                    config: None,
                    create_loader: test_env,
                }
            }

            /// Creates an env w/ default set of plugins
            pub fn test_env() -> EnvLoader {
                let mut loader = EnvLoader {
                    root_dir: std::path::PathBuf::from(".test").join(stringify!($name)),
                    state: State::new(),
                    loaders: std::collections::BTreeSet::new(),
                };
                loader.add_toml_loader::<crate::plugins::Request>();
                loader.add_args_loader::<crate::plugins::RequestArgs>();
                loader
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    test_env!(macro_test);

    #[tokio::test]
    async fn test_test_env_macro() {
        let env = macro_test::loader();

        let mut loader = env.loader();
        let loaded = loader.load(
            &crate::plugins::Request::name(),
            toml_edit::DocumentMut::from_str(
                r#"
url = "https://jsonplaceholder.typicode.com/posts"
        "#,
            )
            .unwrap()
            .as_table(),
        );

        let loaded = loaded.expect("should be able to load");
        let event = loader.state.event(&loaded);
        event.expect("should be able to find request and create event");
    }
}
