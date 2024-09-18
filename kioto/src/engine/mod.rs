mod sequence;
mod load;
mod env;
pub use env::EventConfig;
pub use env::Env;
pub use load::Load;
pub use load::LoadBy;
pub use load::LoadInput;
pub use sequence::Sequence;

use reality::plugin::Event;
use reality::State;

/// An engine manages a collection of events and plugin resources
pub struct Engine {
    /// Engine state which stores plugin resources
    state: State,
    /// Collection of events created by this engine
    events: Vec<Event>
}

impl Engine {
    /// Creates an engine with state
    #[inline]
    pub fn with(state: reality::State) -> Self {
        Engine { state, events: vec![] }
    }

    /// Creates and pushes a plugin event onto the engine
    #[inline]
    pub fn push(&mut self, event: Event) -> reality::Result<()> {
        self.events.push(event);
        Ok(())
    }

    /// Returns an event pushed on to this engine
    #[inline]
    pub fn event(&self, index: usize) -> Option<&Event> {
        self.events.get(index)
    }

    /// Returns a reference to the engine's state
    #[inline]
    pub fn state(&self) -> &State {
        &self.state
    }
}
