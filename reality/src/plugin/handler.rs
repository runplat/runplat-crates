use tracing::debug;

use super::{Bind, Call, Plugin, SpawnWork};
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
    fn wrap_thunk(call: Call) -> Result<SpawnWork> {
        // Receives the original call to plugin R and initiates the work
        let work = Self::Target::thunk(call.clone())?;
        // Finds the resource of the handler
        match call.state.find_plugin(Self::name().path()) {
            Some(handler) => {
                debug!("Found handler resource");
                let handler_call = Call {
                    state: call.state.clone(),
                    item: handler.clone(),
                    fork_fn: Self::fork,
                    cancel: call.state.cancel.child_token(),
                    handle: call.handle.clone(),
                };
                let binding = handler_call.bind::<Self>()?;
                binding.defer(|b, _| async move {
                    debug!("Waiting for target plugin to complete work");
                    // Wait for the original plugin to complete
                    let w = work.await??;
                    w.await?;

                    // Rebind the other plugin
                    let other = call.bind::<Self::Target>()?;

                    // Update the state of the handling plugin w/ the latest state of the other plugin
                    match Self::handle(other, b.clone()) {
                        Ok(_) => {
                            debug!("Calling handler");
                            let completion = Self::call(b)?;
                            let handler_work = completion.await??;
                            handler_work.await
                        }
                        Err(_) => {
                            debug!("Skipping handler");
                            Err(crate::Error::PluginCallSkipped)
                        }
                    }
                })
            }
            None => {
                Ok(work)
            }
        }
    }
}
