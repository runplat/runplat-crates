mod address;
mod call;
mod event;
mod handler;
mod state;
mod thunk;
mod work;
mod messages;

pub mod name;
pub use address::Address;
pub use call::Bind;
pub use call::Call;
pub use event::Event;
pub use handler::Handler;
pub use name::Name;
pub use messages::Broker;
pub use messages::MessageData;
pub use state::State;
pub use thunk::HandlerThunk;
pub use thunk::Thunk;
pub use work::Work;

use runir::repr::Labels;
use runir::{store::Item, Content, Resource};
use semver::Version;
use serde::de::DeserializeOwned;
use clap::{ArgMatches, FromArgMatches};
use crate::CallResult;

/// Type-alias for the a thunk function
pub type ThunkFn = fn(Call) -> CallResult;

/// Type-alias for forking an item
pub type ForkFn = fn(&Item) -> Item;

/// Plugin trait for implementing extensions within the reality framework
pub trait Plugin: Resource + Content + Sized {
    /// Invoked when the thunk assigned to this plugin successfully binds a call to the plugin
    fn call(bind: Bind<Self>) -> CallResult;

    /// Plugin version
    ///
    /// **Recommendation**: Implementation should just use `env!("CARGO_PKG_VERSION")` to avoid confusion
    fn version() -> Version;

    /// Invoked when a binding is created when the thunk is invoked
    fn receive(&self, _data: MessageData) -> Option<Self> {
        None
    }

    /// Invoked when the plugin is being called
    ///
    /// Returns an error if the call cannot be bound to this plugin, or if the underlying plugin call returns an error
    #[inline]
    fn thunk(call: Call) -> CallResult {
        let bind = call.bind::<Self>()?;
        Self::call(bind)
    }

    /// Loads this plugin by toml
    #[inline]
    fn load_by_toml(state: &mut State, toml: &str, labels: Labels) -> std::io::Result<Address>
    where
        Self: DeserializeOwned,
    {
        state.load_by_toml::<Self>(toml, labels)
    }

    /// Loads this plugin by cli args
    #[inline]
    fn load_by_args(state: &mut State, args: &ArgMatches, labels: Labels) -> std::io::Result<Address>
    where
        Self: FromArgMatches,
    {
        state.load_by_args::<Self>(args, labels)
    }

    /// Forks the item
    ///
    /// ## Guidance
    /// Can be overriden to customize how forking is applied to the inner item.
    ///
    /// A plugin can be forked from either a `Call` or `Event` struct. It's important to note
    /// that once an item has been forked in this manner, it can no longer be tracked in state.
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

/// Trait to centralize attributes that must be loaded with a plugin
pub(crate) trait MustLoadHandler: Handler {
    /// Invoked when this plugin is loaded into state
    ///
    /// Loads critical attributes to this plugin, but can be overriden by the plugin
    #[inline]
    fn must_load(put: runir::store::Put<'_, Self>) -> runir::store::Put<'_, Self> {
        put.attr(Thunk::new::<Self>())
            .attr(HandlerThunk::new::<Self>())
            .attr(Self::name())
    }
}

impl<P: Plugin> MustLoad for P {}
impl<P: Handler> MustLoadHandler for P {}
