use clap::Args;
use serde::{Deserialize, Serialize};
use super::Engine;

/// Configures a kioto engine sequence
#[derive(Args)]
pub struct SequenceArgs {

}

#[derive(Serialize, Deserialize)]
pub struct Sequence {
    /// Engine this sequence is executing
    #[serde(skip)]
    _engine: Option<Engine>
}
