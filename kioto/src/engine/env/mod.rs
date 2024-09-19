mod config;
pub use config::EngineConfig;
pub use config::EventConfig;
pub use config::Metadata;

use reality::plugin::Event;
use super::{Load, LoadInput};
use crate::plugins::{Request, RequestArgs};
use clap::FromArgMatches;
use reality::{
    plugin::{Handler, Name},
    repo::Handle,
    Content, Plugin, Repr, Resource, State, Uuid,
};
use serde::de::DeserializeOwned;
use std::{collections::BTreeSet, path::PathBuf};

/// Type-alias for a function that creates an environment
type CreateLoader = fn() -> EnvLoader;

/// Struct containing tools for creating a new environment
/// 
/// The default implementation will automatically include all plugins implemented in this crate
pub struct Env {
    /// Label for this environment
    label: String,
    /// Function for creating a new environment
    create_loader: CreateLoader,
}

impl Env {
    /// Creates a new env repr
    #[inline]
    pub fn new(label: impl Into<String>, create_loader: CreateLoader) -> Self {
        Self {
            label: label.into().trim_matches(['"']).to_string(),
            create_loader,
        }
    }

    /// Tries to initialize from some root directory,
    ///
    /// Will load all config immediately and set the env loader with the loaded config.
    ///
    /// The EnvLoader can then be used to load events from event configurations
    #[inline]
    pub fn env_loader(&self, root: impl Into<PathBuf>) -> std::io::Result<EnvLoader> {
        let mut config = EngineConfig::from_file_system(root, &self.label)?;
        let mut loader = self.loader();
        config
            .load(&mut loader)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("{e:?}")))?;
        loader.config = config;
        Ok(loader)
    }

    /// Creates a new env loader
    #[inline]
    fn loader(&self) -> EnvLoader {
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
            create_loader: default_env,
        }
    }
}

/// Struct containing state for loading an environment
pub struct EnvLoader {
    /// Root directory
    pub root_dir: PathBuf,
    /// State
    pub state: State,
    /// Engine config for this environment
    pub config: EngineConfig,
    /// Map of prepared loaders
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

    /// Adds a handler toml loader to the env loader
    #[inline]
    pub fn add_handler_toml_loader<H: Handler + DeserializeOwned>(&mut self) {
        let h = self
            .state
            .store_mut()
            .put(Load::handler_by_toml::<H>())
            .commit();
        self.loaders.insert((H::name(), h));
    }

    /// Adds a handler arg loader to the env loader
    #[inline]
    pub fn add_handler_args_loader<H: Handler + FromArgMatches>(&mut self) {
        let h = self
            .state
            .store_mut()
            .put(Load::handler_by_args::<H>())
            .commit();
        self.loaders.insert((H::name(), h));
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

    /// Tries to get and configure an event
    #[inline]
    pub fn get_event(&self, config: &EventConfig) -> reality::Result<Event> {
        self.config.config_event(config, self)
    }
}

/// Creates an env w/ default set of plugins
pub fn default_env() -> EnvLoader {
    let mut loader = EnvLoader {
        root_dir: PathBuf::from(".kt").join("default"),
        state: State::new(),
        config: EngineConfig::default(),
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
            use crate::engine::env::EngineConfig;

            /// Creates a new env
            pub fn env() -> Env {
                Env::new(stringify!($name), test_env)
            }

            /// Creates an env w/ default set of plugins
            fn test_env() -> EnvLoader {
                let mut loader = EnvLoader {
                    root_dir: std::path::PathBuf::from(".test").join(stringify!($name)),
                    state: State::new(),
                    config: EngineConfig::default(),
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
        let env = macro_test::env();

        let mut loader = env.loader();
        let loaded = loader.load(
            &crate::plugins::Request::name(),
            toml_edit::DocumentMut::from_str(
                r#"
url = "https://jsonplaceholder.typicode.com/posts"
        "#,
            )
            .unwrap(),
        );

        let loaded = loaded.expect("should be able to load");
        let event = loader.state.event(&loaded);
        event.expect("should be able to find request and create event");
    }
}
