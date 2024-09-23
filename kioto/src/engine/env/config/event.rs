use serde::{Deserialize, Serialize};

/// Configuration for an event to be loaded by an engine
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// Event name from the map of events in the engine config which will handle configuring from
    /// this event config
    pub event: String,
    /// Handler name from the map of handlers in the engine config which will handle configuring from
    /// this event config
    pub handler: Option<String>,
}

impl Config {
    /// Splits for lookup
    #[inline]
    pub fn split_for_lookup(&self) -> (String, Option<String>) {
        (
            self.event.to_lowercase(),
            self.handler.as_ref().map(|h| h.to_lowercase()),
        )
    }
}
