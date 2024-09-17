use super::{Call, Name, Plugin, ThunkFn, Work};
use crate::Result;
use runir::{Repr, Resource};
use serde::Serialize;

/// Attribute created by a plugin
#[derive(Serialize, Clone)]
pub struct Thunk {
    /// Name of the plugin that generated this thunk
    name: Name,
    /// Call function
    #[serde(skip)]
    call: ThunkFn,
}

impl Thunk {
    /// Returns a new thunk from a plugin implementation
    #[inline]
    pub fn new<P: Plugin>() -> Self {
        Thunk {
            name: P::name(),
            call: P::thunk,
        }
    }

    /// Returns the name of the plugin that created this thunk
    #[inline]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Executes the thunk
    #[inline]
    #[must_use = "If the future is not awaited, then the call cannot be executed"]
    pub async fn exec(&self, call: Call) -> Result<Work> {
        (self.call)(call)?.await?
    }
}

impl Repr for Thunk {}
impl Resource for Thunk {}
