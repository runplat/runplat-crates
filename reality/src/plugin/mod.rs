mod thunk;
mod name;
mod state;
mod work;
mod call;
pub use thunk::Thunk;
pub use name::Name;
pub use state::State;
pub use work::Work;
pub use call::Call;
pub use call::Bind;

use runir::Resource;
use std::hash::Hash;
use crate::Result;

/// Type-alias for a join handle that spawns plugin work
pub type SpawnWork = tokio::task::JoinHandle<Result<Work>>;

/// Type-alias for the a thunk function
pub type ThunkFn = fn(Call) -> Result<SpawnWork>;

/// Plugin trait for implementing extensions within the reality framework
pub trait Plugin: Resource + Hash + Sized {
    /// Invoked when the thunk assigned to this plugin successfully binds a call to the plugin
    fn call(bind: Bind<Self>) -> Result<SpawnWork>;

    /// Invoked when the plugin is being called
    ///
    /// Returns an error if the call cannot be bound to this plugin, or if the underlying plugin call returns an error
    fn thunk(call: Call) -> Result<SpawnWork> {
        let bind = call.bind::<Self>()?;
        Self::call(bind)
    }

    /// Name of this plugin
    fn name() -> Name {
        Name::new::<Self>()
    }

    /// Invoked when this plugin is loaded into state
    /// 
    /// Can be overridden to include additional attributes w/ this plugin
    fn load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
        put
    }
}

/// Trait to centralize attributes that must be loaded with a plugin
pub(crate) trait MustLoad : Plugin {
    /// Invoked when this plugin is loaded into state
    /// 
    /// Loads critical attributes to this plugin, but can be overriden by the plugin
    fn must_load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
        put.attr(Thunk::new::<Self>()).attr(Self::name())
    }
}

impl<P: Plugin> MustLoad for P {}
