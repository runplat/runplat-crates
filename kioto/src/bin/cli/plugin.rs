use clap::{Args, Subcommand};
use kioto::plugins::RequestArgs;

/// Arguments for the kioto plugin system
#[derive(Args)]
pub struct PluginArgs {
    /// Plugin to use
    #[clap(subcommand)]
    pub plugin: Plugins,
}

/// Configure or run a plugin
#[derive(Subcommand)]
pub enum Plugins {
    /// HTTPS request plugin
    Request(RequestArgs),
}
