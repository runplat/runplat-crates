use clap::{Args, Subcommand};

/// CLI arguments for interacting with the kioto engine system
#[derive(Args)]
pub struct EngineArgs {
    /// Engine type to interact with
    #[clap(subcommand)]
    engine: Engines,
}

/// Various engine types that can be configured
#[derive(Subcommand)]
pub enum Engines {
    /// Sequence engine is able to define a sequence of events
    Sequence,
}
