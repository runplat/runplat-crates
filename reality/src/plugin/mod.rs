mod address;
mod call;
mod event;
mod name;
mod state;
mod thunk;
mod work;
pub use address::Address;
pub use call::Bind;
pub use call::Call;
pub use event::Event;
pub use name::Name;
use serde::Serialize;
pub use state::State;
pub use thunk::Thunk;
pub use work::Work;

use crate::Result;
use runir::Resource;

/// Type-alias for a join handle that spawns plugin work
pub type SpawnWork = tokio::task::JoinHandle<Result<Work>>;

/// Type-alias for the a thunk function
pub type ThunkFn = fn(Call) -> Result<SpawnWork>;

/// Plugin trait for implementing extensions within the reality framework
pub trait Plugin: Resource + Serialize + Sized {
    /// Invoked when the thunk assigned to this plugin successfully binds a call to the plugin
    fn call(bind: Bind<Self>) -> Result<SpawnWork>;

    /// Invoked when the plugin is being called
    ///
    /// Returns an error if the call cannot be bound to this plugin, or if the underlying plugin call returns an error
    #[inline]
    fn thunk(call: Call) -> Result<SpawnWork> {
        let bind = call.bind::<Self>()?;
        Self::call(bind)
    }

    /// Name of this plugin
    #[inline]
    fn name() -> Name {
        Name::new::<Self>()
    }

    /// Invoked when this plugin is loaded into state
    ///
    /// Can be overridden to include additional attributes w/ this plugin
    #[inline]
    fn load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
        put
    }
}

/// Trait to centralize attributes that must be loaded with a plugin
pub(crate) trait MustLoad: Plugin {
    /// Invoked when this plugin is loaded into state
    ///
    /// Loads critical attributes to this plugin, but can be overriden by the plugin
    #[inline]
    fn must_load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
        put.attr(Thunk::new::<Self>()).attr(Self::name())
    }
}

impl<P: Plugin> MustLoad for P {}
