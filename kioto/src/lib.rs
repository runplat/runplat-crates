//! # Kioto Engine
//!
//! Provides an engine built on top of the `reality` plugin framework

use reality::plugin::Name;

pub mod engine;
pub mod plugins;

/// Type-alias for a crate error
pub type Result<T> = std::result::Result<T, Errors>;

#[derive(Debug)]
pub enum Errors {
    /// Error occured in `reality`
    Reality(reality::Error),
    /// Error returned when a plugin could not be loaded for an event
    PluginLoadError(PluginLoadErrors),
}

#[derive(Debug)]
pub enum PluginLoadErrors {
    MissingFile(CouldNotLoadPlugin),
    CouldNotReadFile {
        error: CouldNotLoadPlugin,
        io: std::io::Error,
    },
}

/// Error returned when a plugin could not be loaded
#[derive(Debug)]
pub struct CouldNotLoadPlugin {
    /// Event that was being loaded
    pub event: String,
    /// Name of the plugin that was trying to be loaded
    pub name: Name,
}

impl CouldNotLoadPlugin {
    /// Creates a new could not load plugin error
    #[inline]
    pub fn new(event: impl Into<String>, name: Name) -> Self {
        Self { event: event.into(), name }
    }
}

impl From<reality::Error> for Errors {
    fn from(value: reality::Error) -> Self {
        Self::Reality(value)
    }
}
