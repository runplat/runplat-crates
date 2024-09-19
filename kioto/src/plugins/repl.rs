use clap::ArgMatches;
use reality::{
    plugin::{Bind, Handler, SpawnWork},
    Content, Plugin, Repr, Resource, Uuid,
};

use super::utils::with_cancel;
use tokio::io::{stdin, AsyncBufReadExt, BufReader};

/// Repl plugin is a handler plugin that can be used to interact and test other plugins
pub struct Repl<T: Plugin> {
    /// Handler target for this repl plugin
    target: Option<Bind<T>>,
}

impl<T: Plugin> Plugin for Repl<T> {
    fn call(bind: reality::plugin::Bind<Self>) -> reality::Result<reality::plugin::SpawnWork> {
        if let Some(_target_repl) = bind.plugin()?.target.as_ref().and_then(|t| {
            t.item()
                .attributes()
                .get::<ReplInterface<T>>()
                .map(|ri| (ri, t.clone()))
        }) {
            bind.defer(|_, ct| async move {
                loop {
                    let ct = ct.clone();
                    // TODO: REPL loop here
                    let (repl, target_bind) = _target_repl.clone();
                    let read = target_bind.handle().clone();
                    let read = read.spawn_blocking(move || async move {
                        match BufReader::new(stdin()).lines().next_line().await {
                            Ok(Some(line)) => {
                                if let Some(args) = shlex::split(&line) {
                                    let matches = repl.command.clone().get_matches_from(args);
                                    let work = (repl.eval)(matches, &target_bind)?;
                                    let work = work.await??;
                                    work.await?;
                                    Ok(true)
                                } else {
                                    Err(reality::Error::PluginCallCancelled)
                                }
                            }
                            Ok(None) => Ok(false),
                            Err(_) => Err(reality::Error::PluginCallCancelled),
                        }
                    });
                    let result = with_cancel(ct)
                        .run(async move { read.await?.await }, |r| match r {
                            Ok(true) => Ok(()),
                            _ => Err(reality::Error::PluginCallCancelled),
                        })
                        .await;

                    if result.is_err() {
                        return Ok(());
                    }
                }
            })
        } else {
            Err(reality::Error::PluginCallSkipped)
        }
    }

    fn version() -> reality::Version {
        reality::Version::new(0, 1, 0)
    }
}

impl<T: Plugin> Handler for Repl<T> {
    type Target = T;

    fn handle(
        other: reality::plugin::Bind<Self::Target>,
        mut handler: reality::plugin::Bind<Self>,
    ) -> reality::Result<()> {
        let repl = handler.plugin_mut()?;
        repl.target = Some(other);
        Ok(())
    }
}

impl<T: Plugin> Resource for Repl<T> {}
impl<T: Plugin> Content for Repl<T> {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        Uuid::new_v4()
    }
}

pub struct ReplInterface<T: Plugin> {
    command: clap::Command,
    eval: fn(clap::ArgMatches, &Bind<T>) -> reality::Result<SpawnWork>,
}

impl<T: Plugin + ReplEval> ReplInterface<T> {
    /// Creates a new repl interface based on a type that implements the ReplEval trait
    #[inline]
    pub fn new() -> Self {
        ReplInterface {
            command: T::command().multicall(true),
            eval: T::eval,
        }
    }
}

pub trait ReplEval: Plugin {
    /// Command that configures the repl
    fn command() -> clap::Command;

    /// Evaluates the next set of arg matches
    fn eval(next: ArgMatches, call: &Bind<Self>) -> reality::Result<SpawnWork>;
}

impl<T: Plugin> Resource for ReplInterface<T> {}
impl<T: Plugin> Repr for ReplInterface<T> {}
impl<T: Plugin> Content for ReplInterface<T> {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        todo!()
    }
}
