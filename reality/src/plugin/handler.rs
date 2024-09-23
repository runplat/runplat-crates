use clap::{ArgMatches, FromArgMatches};
use runir::repr::Labels;
use serde::de::DeserializeOwned;
use tracing::debug;

use super::{Address, Bind, Call, Plugin, State, Work};
use crate::Result;

/// Trait for a plugin that can be called to handle some input plugin
pub trait Handler: Plugin {
    /// Target plugin to handle
    type Target: Plugin;

    /// Called after the other plugin has completed successfully
    ///
    /// Returns an error if handling should be skipped
    fn handle(other: Bind<Self::Target>, handler: Bind<Self>) -> Result<()>;

    /// Thunk function that wraps another thunk function in order to sequence the handler to execute
    /// after the call of the plugin it is handling
    fn wrap_thunk(call: Call) -> Result<Work> {
        // Receives the original call to plugin R and initiates the work
        let work = Self::Target::thunk(call.clone())?;
        let handler = call.handler().and_then(|a| call.state.find_plugin(a));
        // Finds the resource of the handler
        match handler.or_else(|| call.state.find_plugin(Self::name().path())) {
            Some(handler) => {
                debug!("Found handler resource");
                let handler_call = Call {
                    state: call.state.clone(),
                    item: handler.clone(),
                    fork_fn: Self::fork,
                    cancel: call.state.cancel.child_token(),
                    runtime: call.runtime.clone(),
                    handler: None,
                };
                let binding = handler_call.bind::<Self>()?;
                binding.defer(|b, _| async move {
                    debug!("Waiting for target plugin to complete work");
                    // Wait for the original plugin to complete
                    work.await?;
                    // Rebind the other plugin
                    let other = call.bind::<Self::Target>()?;

                    // Update the state of the handling plugin w/ the latest state of the other plugin
                    match Self::handle(other, b.clone()) {
                        Ok(_) => {
                            debug!("Calling handler");
                            Self::call(b)?.await
                        }
                        Err(_) => {
                            debug!("Skipping handler");
                            Err(crate::Error::PluginCallSkipped)
                        }
                    }
                })
            }
            None => Ok(work),
        }
    }

    /// Loads this plugin by toml
    #[inline]
    fn load_handler_by_toml(
        state: &mut State,
        toml: &str,
        labels: Labels,
    ) -> std::io::Result<Address>
    where
        Self: DeserializeOwned,
    {
        state.load_handler_by_toml::<Self>(toml, labels)
    }

    /// Loads this plugin by cli args
    #[inline]
    fn load_handler_by_args(
        state: &mut State,
        args: &ArgMatches,
        labels: Labels,
    ) -> std::io::Result<Address>
    where
        Self: FromArgMatches,
    {
        state.load_handler_by_args::<Self>(args, labels)
    }
}
