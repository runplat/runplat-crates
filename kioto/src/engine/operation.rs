use crate::plugins::utils::with_cancel;
use plugin::Bind;
use reality::*;
use serde::{Deserialize, Serialize};

use super::{default_env, Engine, Env, EventConfig, Metadata};

/// Plugin for executing a list of events
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
    #[serde(rename = "_kt-meta")]
    metadata: Option<Metadata>,
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
        .plugin()?
        .metadata
        .as_ref()
        .map(|m| m.split_for_env_loader())
        .unwrap_or_else(|| ("default".to_string(), std::env::current_dir()));

    // Build the engine if it hasn't already been built
    let loader = Env::new(env, default_env).env_loader(root_dir?)?;
    let mut engine = Engine::with(loader.state.clone());
    for e in binding.plugin()?.events.iter() {
        let event = loader.get_event(e)?;
        engine.push(event)?;
    }
    binding.plugin_mut()?.engine = Some(engine);

    binding.defer(|i, ct| async move {
        match i.plugin()?.engine.as_ref() {
            Some(engine) => {
                for e in engine.events.iter() {
                    let (f, _) = e.fork();
                    with_cancel(ct.clone()).run(f.start(), |r| r).await?;
                }
                Ok(())
            }
            None => Err(reality::Error::PluginCallSkipped),
        }
    })
}
