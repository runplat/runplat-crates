use clap::ArgMatches;
use reality::{
    plugin::{Bind, Handler},
    CallResult, Content, Plugin, Repr, Resource, Uuid,
};

use super::utils::with_cancel;

/// Repl plugin is a handler plugin that can be used to interact and test other plugins
pub struct Repl<T: Plugin + ReplEval> {
    /// Handler target for this repl plugin
    target: Option<Bind<T>>,
}

impl<T: Plugin + ReplEval> Plugin for Repl<T> {
    fn call(bind: reality::plugin::Bind<Self>) -> CallResult {
        if let Some(_target_repl) = bind.plugin()?.target.as_ref().and_then(|t| {
            bind.item()
                .attributes()
                .get::<ReplInterface<T>>()
                .map(|ri| (ri, t.clone()))
        }) {
            bind.defer(|_, ct| async move {
                loop {
                    let ct = ct.clone();
                    let (repl, target_bind) = _target_repl.clone();
                    // READ
                    let read_bind = target_bind.clone();
                    let read = target_bind.handle().clone();
                    let read = read.spawn_blocking(move || {
                        if let Some(Ok(line)) = std::io::stdin().lines().next() {
                            // EVAL
                            if let Some(args) = shlex::split(&line) {
                                let matches = (repl.command)().get_matches_from(args);
                                (repl.eval)(matches, &read_bind)?;
                                Ok(true)
                            } else {
                                Err(reality::Error::PluginCallCancelled)
                            }
                        } else {
                            Ok(false)
                        }
                    });

                    let result = with_cancel(ct)
                        .run(
                            async move {
                                let should_eval = read.await?;
                                if should_eval? {
                                    T::call(target_bind)?.await
                                } else {
                                    Err(reality::Error::PluginCallCancelled)
                                }
                            },
                            |r| match r {
                                Ok(_) => Ok(()),
                                _ => Err(reality::Error::PluginCallCancelled),
                            },
                        )
                        .await;

                    // LOOP -- TODO, better error handling
                    if result.is_err() {
                        return Ok(());
                    }
                }
            })
        } else {
            eprintln!("REPL interface is not installed");
            Err(reality::Error::PluginCallSkipped)
        }
    }

    fn version() -> reality::Version {
        reality::Version::new(0, 1, 0)
    }

    fn load(put: reality::runir::store::Put<'_, Self>) -> reality::runir::store::Put<'_, Self> {
        put.attr(ReplInterface::<T>::new())
    }
}

impl<T: Plugin + ReplEval> Handler for Repl<T> {
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

impl<T: Plugin + ReplEval> Default for Repl<T> {
    fn default() -> Self {
        Self { target: None }
    }
}

impl<T: Plugin + ReplEval> Resource for Repl<T> {}
impl<T: Plugin + ReplEval> Content for Repl<T> {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        Uuid::new_v4()
    }
}

pub struct ReplInterface<T: Plugin> {
    command: fn() -> clap::Command,
    eval: fn(clap::ArgMatches, &Bind<T>) -> reality::Result<()>,
}

impl<T: Plugin + ReplEval> ReplInterface<T> {
    /// Creates a new repl interface based on a type that implements the ReplEval trait
    #[inline]
    pub fn new() -> Self {
        ReplInterface {
            command: T::command,
            eval: T::eval,
        }
    }
}

pub trait ReplEval: Plugin {
    /// Command that configures the repl
    fn command() -> clap::Command;

    /// Evaluates the next set of arg matches
    fn eval(next: ArgMatches, call: &Bind<Self>) -> reality::Result<()>;
}

impl<T: Plugin> Resource for ReplInterface<T> {}
impl<T: Plugin> Repr for ReplInterface<T> {}
impl<T: Plugin> Content for ReplInterface<T> {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        reality::uuid::Uuid::new_v4()
    }
}

#[cfg(test)]
mod tests {
    use super::ReplEval;
    use crate::plugins::repl::Repl;
    use clap::{Arg, ArgAction};
    use reality::{repr::Labels, CallResult, Content, Plugin, Resource, State, Uuid};
    use tokio::io::AsyncWriteExt;

    #[ignore = "would block"]
    #[tokio::test]
    async fn test_example_repl_impl() {
        let mut state = State::new();
        let address = state.load(
            Echo {
                message: String::new(),
            },
            Labels::default(),
        );
        let _ = state.load_handler(Repl::<Echo>::default(), Labels::default());

        let mut event = state.event(&address).unwrap();
        event.with_handler::<Repl<Echo>>().unwrap();
        event.start().await.unwrap();
    }

    ///
    struct Echo {
        message: String,
    }

    impl Plugin for Echo {
        fn call(bind: reality::plugin::Bind<Self>) -> CallResult {
            bind.work(|p, _| {
                let message = p.message.to_string();
                async move {
                    if !message.is_empty() {
                        tokio::io::stderr()
                            .write_all(message.trim().as_bytes())
                            .await
                            .unwrap();
                        tokio::io::stderr().write_all(b"\r\n").await.unwrap();
                    }
                    Ok(())
                }
            })
        }

        fn version() -> reality::Version {
            reality::Version::new(0, 0, 0)
        }
    }

    impl ReplEval for Echo {
        fn command() -> clap::Command {
            clap::Command::new("echo")
                .subcommand(
                    clap::Command::new("echo").arg(
                        Arg::new("message")
                            .action(ArgAction::Append)
                            .value_delimiter(' '),
                    ),
                )
                .subcommand(clap::Command::new("exit"))
                .multicall(true)
        }

        fn eval(
            mut next: clap::ArgMatches,
            call: &reality::plugin::Bind<Self>,
        ) -> reality::Result<()> {
            match next.remove_subcommand() {
                Some(m) => match (m.0.as_str(), m.1) {
                    ("echo", mut matches) => {
                        if let Some(message) = matches.remove_many::<String>("message") {
                            call.clone().plugin_mut()?.message =
                                message.collect::<Vec<_>>().join(" ");
                            Ok(())
                        } else {
                            Err(reality::Error::PluginCallCancelled)
                        }
                    }
                    ("exit", _) => Err(reality::Error::PluginCallCancelled),
                    _ => {
                        Echo::command().print_help().unwrap();
                        Ok(())
                    }
                },
                None => Err(reality::Error::PluginCallCancelled),
            }
        }
    }

    impl Resource for Echo {}
    impl Content for Echo {
        fn state_uuid(&self) -> reality::uuid::Uuid {
            Uuid::new_v4()
        }
    }
}
