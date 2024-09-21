mod config;
pub use config::BuildMetadata;
pub use config::EngineConfig;
pub use config::EventConfig;
pub use config::LoaderMetadata;
pub use config::Metadata;

mod build;
pub use build::Builder as EnvBuilder;

use super::{Load, LoadInput, Operation};
use crate::plugins::{Request, RequestArgs};
use clap::FromArgMatches;
use reality::{
    plugin::{Event, Handler, Name},
    repo::Handle,
    Plugin, State,
};
use serde::de::DeserializeOwned;
use std::{collections::BTreeSet, path::PathBuf};

/// Creates an env w/ default set of plugin loaders
pub fn default_create_env(label: String, root_dir: PathBuf) -> Env {
    let mut loader = Env {
        label,
        root_dir,
        state: State::new(),
        config: EngineConfig::default(),
        loaders: BTreeSet::new(),
    };
    loader.add_toml_loader::<Operation>();
    loader.add_toml_loader::<Request>();
    loader.add_args_loader::<RequestArgs>();
    loader
}

/// Struct containing environment state which can be used to load plugins for building engine state
pub struct Env {
    /// Env label
    pub label: String,
    /// Root directory
    pub root_dir: PathBuf,
    /// State
    pub state: State,
    /// Engine config for this environment
    pub config: EngineConfig,
    /// Map of prepared loaders
    pub loaders: BTreeSet<(Name, Handle)>,
}

impl Env {
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

    /// Tries to create and configure an event
    #[inline]
    pub fn create_event(&self, config: &EventConfig) -> reality::Result<Event> {
        self.config.configure_event(config, self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_test_env_macro() {
        let env = EnvBuilder::default_env("test");

        let mut loader = env.load_env(PathBuf::from(".test")).unwrap();
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

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_env_build() {
        let default = build::Builder::new("test_operation", default_create_env);
        
        // Clean up env
        let target = PathBuf::from(".test").join("test_operatoin");
        if target.exists() {
            std::fs::remove_dir_all(target).unwrap();
        }

        default.build_env("tests/data", ".test").expect("should be able to build");
        default.load_env(".test").expect("should be able to load");
    }
}
