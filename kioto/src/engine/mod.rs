mod event;
pub use event::Event;

/// An engine manages access and configuring a state instance
pub struct Engine {
    /// Engine state which stores plugin resources
    state: reality::State,
}

impl Engine {
    /// Creates an engine with state
    #[inline]
    pub fn with(state: reality::State) -> Self {
        Engine { state }
    }
}