use reality::{plugin::Address, runir::Resource, Plugin};
use serde::Serialize;

/// Event represents the smallest unit of work
///
/// An event contains a list of plugins resources that it will access, and is itself a plugin and resource as well.
///
/// An event maintains a lifecycle of roughly `Created -> Executing -> Completed`.
///
/// When an event reaches `Completed`, it will reset to it's state at `Created`.
#[derive(Serialize)]
pub struct Event {
    /// List of plugins associated to this event
    plugins: Vec<Address>,
    /// Instant this event started
    #[serde(skip)]
    started: Option<tokio::time::Instant>,
}

impl Plugin for Event {
    fn call(mut bind: reality::plugin::Bind<Self>) -> reality::Result<reality::plugin::SpawnWork> {
        let event = bind.plugin_mut()?;
        event.started = Some(tokio::time::Instant::now());

        Ok(bind.work_mut(|p, ct| {
            async { Ok(()) }
        }))
    }
}

impl Resource for Event {}
