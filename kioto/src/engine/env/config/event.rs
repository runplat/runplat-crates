use serde::{Deserialize, Serialize};

/// Settings for an event that should be created
#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Reference of the plugin to create this event with
    plugin: String,
}