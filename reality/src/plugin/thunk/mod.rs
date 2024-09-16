mod context;
pub use context::Context;
use runir::{repr::Repr, Resource};

use super::{AsyncContext, Namespace, Plugin};

/// Type-alias for the call function of a plugin
type CallFn = fn(Context) -> Option<AsyncContext>;

/// Call by name primitive
#[derive(Hash, Clone)]
pub struct Thunk {
    /// Namespace for this thunk
    namespace: Namespace,
    /// Call function
    call: CallFn
}

impl Thunk {
    /// Returns a new thunk from a plugin implementation
    pub fn new<P: Plugin>() -> Self {
        Thunk { namespace: P::namespace(), call: P::call }
    }

    /// Returns the address representing this thunk
    pub fn address(&self) -> &str {
        ""
    }
}

impl Repr for Thunk {}
impl Resource for Thunk {}