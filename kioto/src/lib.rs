//! # Kioto Engine
//!
//! Provides an engine built on top of the `reality` plugin framework

pub mod engine;
pub mod plugins;

/// Type-alias for a crate error
pub type Result<T> = std::result::Result<T, Errors>;

pub enum Errors {
    /// Error occured in `reality`
    Reality(reality::Error)
}

impl From<reality::Error> for Errors {
    fn from(value: reality::Error) -> Self {
        Self::Reality(value)
    }
}