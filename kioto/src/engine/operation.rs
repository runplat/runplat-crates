use crate::plugins::utils::with_cancel;
use plugin::Bind;
use reality::*;
use runplat_macros::kt_metadata;
use serde::{Deserialize, Serialize};

use super::{Engine, EnvBuilder, EventConfig, Metadata};

/// Plugin for executing a list of events
#[kt_metadata(build, loader)]
#[derive(Plugin, Serialize, Deserialize)]
#[reality(
    call = execute_operation,
    content_from = BincodeContent
)]
pub struct Operation {
    /// List of event config
    events: Vec<EventConfig>,
    /// Engine this sequence is executing
    #[serde(skip)]
    engine: Option<Engine>,
}

impl Operation {
    /// Takes the inner engine
    #[inline]
    pub fn take_engine(&mut self) -> Option<Engine> {
        self.engine.take()
    }
}

fn execute_operation(mut binding: Bind<Operation>) -> CallResult {
    // Resolve the current env and root directory
    let (env, root_dir) = binding
        .receiver()?
        .loader()
        .as_ref()
        .map(|m| m.split_for_env_loader())
        .unwrap_or_else(|| ("default".to_string(), std::env::current_dir()));

    // Build the engine if it hasn't already been built
    let loader = EnvBuilder::default_env(env).load_env(root_dir?)?;
    let mut engine = Engine::with(loader.state.clone());
    for e in binding.receiver()?.events.iter() {
        let event = loader.create_event(e)?;
        engine.push(event)?;
    }
    binding.update()?.engine = Some(engine);
    binding.defer(|i, ct| async move {
        match i.receiver()?.engine.as_ref() {
            Some(engine) => {
                for e in engine.events.iter() {
                    let (f, _) = e.fork();
                    with_cancel(ct.clone()).run(f.start()).await??;
                }
                Ok(())
            }
            None => Err(reality::Error::PluginCallSkipped),
        }
    })
}
