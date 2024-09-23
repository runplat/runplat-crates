//! # Kioto Engine
//!
//! Provides an engine built on top of the `reality` plugin framework

use reality::plugin::Name;

pub use runplat_macros::kt_metadata;
extern crate self as kioto;

pub mod engine;
pub mod plugins;

/// Name of the table for BuildMetadata
pub const KT_BUILD_METADATA_TABLE: &str = "-kt-build";

/// Name of the table for LoaderMetadata
///
/// **Note**: Loader metadata is created by the loader, will be ignored if set by the user.
pub const KT_LOADER_METADATA_TABLE: &str = "-kt-loader";

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
    /// Error returned when a src file is missing
    MissingFile(CouldNotLoadPlugin),
    /// Error returned when a file could not be loaded
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
        Self {
            event: event.into(),
            name,
        }
    }
}

impl From<reality::Error> for Errors {
    fn from(value: reality::Error) -> Self {
        Self::Reality(value)
    }
}
