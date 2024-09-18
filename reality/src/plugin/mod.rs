mod address;
mod call;
mod handler;
mod event;
mod name;
mod state;
mod thunk;
mod work;

pub use handler::Handler;
pub use address::Address;
pub use call::Bind;
pub use call::Call;
use clap::ArgMatches;
use clap::FromArgMatches;
pub use event::Event;
pub use name::Name;
use runir::store::Item;
use runir::Content;
use semver::Version;
use serde::de::DeserializeOwned;
pub use state::State;
pub use thunk::Thunk;
pub use work::Work;

use crate::Result;
use runir::Resource;

/// Type-alias for a join handle that spawns plugin work
pub type SpawnWork = tokio::task::JoinHandle<Result<Work>>;

/// Type-alias for the a thunk function
pub type ThunkFn = fn(Call) -> Result<SpawnWork>;

/// Type-alias for forking an item
pub type ForkFn = fn(&Item) -> Item;

/// Plugin trait for implementing extensions within the reality framework
pub trait Plugin: Resource + Content + Sized {
    /// Invoked when the thunk assigned to this plugin successfully binds a call to the plugin
    fn call(bind: Bind<Self>) -> Result<SpawnWork>;

    /// Plugin version
    ///
    /// **Recommendation**: Implementation should just use `env!("CARGO_PKG_VERSION")` to avoid confusion
    fn version() -> Version;

    /// Invoked when the plugin is being called
    ///
    /// Returns an error if the call cannot be bound to this plugin, or if the underlying plugin call returns an error
    #[inline]
    fn thunk(call: Call) -> Result<SpawnWork> {
        let bind = call.bind::<Self>()?;
        Self::call(bind)
    }

    /// Loads this plugin by toml
    #[inline]
    fn load_by_toml(state: &mut State, toml: &str) -> std::io::Result<Address>
    where
        Self: DeserializeOwned,
    {
        state.load_by_toml::<Self>(toml)
    }

    /// Loads this plugin by cli args
    #[inline]
    fn load_by_args(state: &mut State, args: &ArgMatches) -> std::io::Result<Address>
    where
        Self: FromArgMatches,
    {
        state.load_by_args::<Self>(args)
    }

    /// Forks the item
    /// 
    /// ## Guidance
    /// Can be overriden to customize how forking is applied to the inner item.
    /// 
    /// A plugin can be forked from either a `Call` or `Event` struct
    #[inline]
    fn fork(item: &Item) -> Item {
        item.clone()
    }

    /// Name of this plugin
    #[inline]
    fn name() -> Name {
        Name::new::<Self>()
    }

    /// Version of the framework
    #[inline]
    fn framework() -> (&'static str, &'static str) {
        (env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
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
