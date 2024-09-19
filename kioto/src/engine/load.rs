use clap::{ArgMatches, FromArgMatches};
use reality::{
    plugin::{Address, Name},
    Content, Plugin, Repr, Resource, State,
};
use serde::de::DeserializeOwned;
use std::io::Error;

/// Type-alias for a function to load a plugin by toml
type LoadByToml = fn(&mut State, &str) -> std::io::Result<Address>;

/// Type-alias for a function to load a plugin by arg matches
type LoadByArgs = fn(&mut State, &ArgMatches) -> std::io::Result<Address>;

/// Resource for loading a plugin
#[derive(Clone)]
pub struct Load {
    /// Name of the plugin this resource is configured to load
    name: Name,
    /// Load function
    load: LoadBy,
}

/// Enumeration of load plugin functions
#[derive(Clone)]
pub enum LoadBy {
    /// Load plugin by toml
    Toml(LoadByToml),
    /// Load plugin by cli arg matches
    Args(LoadByArgs),
}

/// Enumeration of load plugin input
pub enum LoadInput {
    /// Toml input
    Toml(String),
    /// Arg matches
    Args(ArgMatches),
}

impl Load {
    /// Loads a plugin from input
    #[inline]
    pub fn load<'load>(
        &'load self,
        state: &mut State,
        input: impl Into<LoadInput>,
    ) -> std::io::Result<Address> {
        let input = input.into();
        match (&self.load, input) {
            (LoadBy::Toml(load_toml), LoadInput::Toml(input_toml)) => load_toml(state, &input_toml),
            (LoadBy::Args(load_args), LoadInput::Args(input_args)) => load_args(state, &input_args),
            _ => Err(Error::new(
                std::io::ErrorKind::InvalidInput,
                "Could not load input with provided input settings",
            )),
        }
    }

    /// Creates a load resource for a plugin to load by cli arg matches
    #[inline]
    pub fn by_args<P>() -> Self
    where
        P: Plugin + FromArgMatches,
    {
        Self {
            name: P::name(),
            load: LoadBy::Args(P::load_by_args),
        }
    }

    /// Creates a load resource for a plugin to load by toml
    #[inline]
    pub fn by_toml<P>() -> Self
    where
        P: Plugin + DeserializeOwned,
    {
        Self {
            name: P::name(),
            load: LoadBy::Toml(P::load_by_toml),
        }
    }

    /// Returns the name of the plugin this resource loads
    #[inline]
    pub fn name(&self) -> &Name {
        &self.name
    }
}

impl From<ArgMatches> for LoadInput {
    fn from(value: ArgMatches) -> Self {
        LoadInput::Args(value)
    }
}

impl<'l> From<&'l toml_edit::Table> for LoadInput {
    fn from(value: &'l toml_edit::Table) -> Self {
        LoadInput::Toml(value.to_string())
    }
}

impl Content for Load {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        let mut crc = reality::content::crc().digest();
        crc.update(self.name.full_plugin_ref().as_bytes());
        crc.update(std::any::type_name::<Self>().as_bytes());
        crc.update(b"load");
        reality::Uuid::from_u64_pair(crc.finalize(), 0)
    }
}
impl Resource for Load {}
impl Repr for Load {}
