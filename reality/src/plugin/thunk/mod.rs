mod handler;
pub use handler::HandlerThunk;

use super::{Call, ForkFn, Handler, Name, Plugin, ThunkFn};
use crate::Result;
use runir::{Content, Repr, Resource};

/// Attribute created by a plugin
#[derive(Clone)]
pub struct Thunk {
    /// Name of the plugin that generated this thunk
    name: Name,
    /// Thunk function
    thunk: ThunkFn,
    /// Fork function
    fork: ForkFn,
}

impl Thunk {
    /// Returns a new thunk from a plugin implementation
    #[inline]
    pub fn new<P: Plugin>() -> Self {
        Thunk {
            name: P::name(),
            thunk: P::thunk,
            fork: P::fork,
        }
    }

    /// Returns a thunk for a plugin handler implementation
    #[inline]
    pub fn handler<H: Handler>() -> Self {
        Self {
            name: H::name(),
            thunk: H::wrap_thunk,
            fork: H::fork,
        }
    }

    /// Returns the thunk fn
    #[inline]
    pub fn thunk_fn(&self) -> ThunkFn {
        self.thunk
    }

    /// Returns the fork function set for this thunk
    #[inline]
    pub fn fork_fn(&self) -> ForkFn {
        self.fork
    }

    /// Returns the name of the plugin that created this thunk
    #[inline]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Executes the thunk
    #[inline]
    #[must_use = "If the future is not awaited, then the call cannot be executed"]
    pub async fn exec(&self, call: Call) -> Result<()> {
        (self.thunk)(call)?.await
    }
}

impl Repr for Thunk {}
impl Resource for Thunk {}

impl Content for Thunk {
    fn state_uuid(&self) -> uuid::Uuid {
        let mut crc = runir::content::crc().digest();
        crc.update(self.name.full_plugin_ref().as_bytes());
        crc.update(stringify!(Thunk).as_bytes());
        uuid::Uuid::from_u64_pair(crc.finalize(), 0)
    }
}
