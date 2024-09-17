use super::{Call, Name, Plugin, ThunkFn, Work};
use crate::Result;
use runir::{Content, Repr, Resource};

/// Attribute created by a plugin
#[derive(Clone)]
pub struct Thunk {
    /// Name of the plugin that generated this thunk
    name: Name,
    /// Call function
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

impl Content for Thunk {
    fn state_uuid(&self) -> uuid::Uuid {
        let mut crc = runir::content::crc().digest();
        crc.update(self.name.full_plugin_ref().as_bytes());
        crc.update(&(self.call as u64).to_be_bytes());
        uuid::Uuid::from_u64_pair(crc.finalize(), 0)
    }
}