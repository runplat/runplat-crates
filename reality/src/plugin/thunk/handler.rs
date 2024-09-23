use std::any::TypeId;

use crate::plugin::Handler;

use super::Thunk;
use runir::*;

/// Handler thunk type
///
/// Can be used to set the handler thunk on an event without the need for generics
#[derive(Clone)]
pub struct HandlerThunk {
    /// Target type name
    target_name: &'static str,
    /// Target type id
    target: TypeId,
    /// wrap_thunk for the handler thunk
    wrap_thunk: Thunk,
}

impl HandlerThunk {
    /// Creates a new handler thunk repr
    #[inline]
    pub fn new<H: Handler>() -> Self {
        Self {
            target_name: std::any::type_name::<H::Target>(),
            target: std::any::TypeId::of::<H::Target>(),
            wrap_thunk: Thunk::handler::<H>(),
        }
    }

    /// Returns the inner wrap_thunk
    #[inline]
    pub fn thunk(&self) -> Thunk {
        self.wrap_thunk.clone()
    }

    /// Returns true if `T` matches the target type this handler targets
    #[inline]
    pub fn is_target<T: 'static>(&self) -> bool {
        std::any::TypeId::of::<T>() == self.target
    }

    /// Returns the current target type id
    #[inline]
    pub fn target_type(&self) -> TypeId {
        self.target
    }
}

impl Repr for HandlerThunk {}
impl Resource for HandlerThunk {}

impl Content for HandlerThunk {
    fn state_uuid(&self) -> uuid::Uuid {
        let mut crc = runir::content::crc().digest();
        crc.update(self.target_name.as_bytes());
        crc.update(self.wrap_thunk.name.full_plugin_ref().as_bytes());
        crc.update(stringify!(HandlerThunk).as_bytes());
        uuid::Uuid::from_u64_pair(crc.finalize(), 0)
    }
}
