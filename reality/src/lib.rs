//! # Reality framework
//!
//! This framework provides a trait `Plugin` which can be used as a building block for an plugin extension system.
//!
//! This framework is based on a concept known as "thunks" which is a call by name architecture. The main entrypoint is the
//! type `State`, which is used to register and call plugins.
//!
//! The framework is built on top of the tokio runtime system, and an effort is made to make all components thread-safe by default.

pub mod plugin;
pub use plugin::Plugin;
pub use plugin::State;

/// Re-export runir since it will be required for extending reality
pub use runir;
pub use runir::*;

/// Type-alias for this crates main result type
pub type Result<T> = std::result::Result<T, Error>;

/// Enum of error variants produced by this library
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Error {
    /// Error when a join handle can not run to completion, analagous to tokio::runtime::JoinError
    TaskError { is_panic: bool, is_cancel: bool },
    /// Error returned when a plugin could not be loaded from a path
    LoadPluginError,
    /// Error when a plugin cannot be found in the current state
    PluginNotFound,
    /// Error returned when casting a dynamic pointer to a plugin
    PluginMismatch,
    /// Error returned when a plugin call is cancelled
    PluginCallCancelled,
    /// Error returned when a plugin call is skipped by the plugin
    PluginCallSkipped,
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::TaskError {
            is_panic: e.is_panic(),
            is_cancel: e.is_cancelled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use plugin::{Bind, Call, Plugin, State};
    use runir::Resource;
    use serde::{Deserialize, Serialize};
    use std::{
        env,
        hash::Hash,
        sync::{Arc, OnceLock},
        time::Duration,
    };
    use tokio_util::sync::CancellationToken;

    #[derive(Deserialize, Serialize, Hash)]
    struct TomlPlugin {
        name: String,
    }

    impl Plugin for TomlPlugin {
        fn call(_: Bind<Self>) -> Result<plugin::SpawnWork> {
            Err(Error::PluginCallSkipped)
        }
    }
    impl Resource for TomlPlugin {}

    #[tokio::test]
    async fn test_plugin_load_toml() {
        let mut state = State::new();

        let toml = toml::to_string(&TomlPlugin {
            name: String::from("hello world"),
        });

        state
            .load_toml::<TomlPlugin>(&toml.expect("should be able to serialize"))
            .expect("should be able to load");

        let addr = state
            .addresses()
            .next()
            .expect("should have loaded the plugin");
        assert_eq!(
            "reality/0.1.0/tests/tomlplugin/18a31368b3e2b8b3",
            addr.to_string()
        );
    }

    #[tokio::test]
    async fn test_plugin_replacement() {
        let mut state = State::new();
        state.register(TestPlugin {
            skip: false,
            called: Arc::new(OnceLock::new()),
            call_mut: false,
        });

        state.register(TestPlugin {
            skip: true,
            called: Arc::new(OnceLock::new()),
            call_mut: false,
        });
        let mut addresses = state.addresses();
        assert_eq!(
            "reality/0.1.0/tests/testplugin/920dbec49bc792e2",
            addresses.next().expect("should have address").to_string()
        );
        assert_eq!(
            "reality/0.1.0/tests/testplugin/d5e704c13717f385",
            addresses.next().expect("should have address").to_string()
        );
        assert_eq!(2, state.addresses().count());
    }

    #[tokio::test]
    async fn test_plugin_work_cancel() {
        let mut state = State::new();
        state.register(TestPlugin {
            skip: false,
            called: Arc::new(OnceLock::new()),
            call_mut: false,
        });

        let path = TestPlugin::name();
        let (f, cancel) = state.spawn(path.path()).expect("should spawn");

        cancel.cancel();
        assert_eq!(
            Error::PluginCallCancelled,
            f.await.expect_err("should be cancelled")
        );
    }

    #[tokio::test]
    async fn test_join_error_conversion() {
        let handle = tokio::runtime::Handle::current();

        let jh = handle.spawn(async {});
        jh.abort();
        assert_eq!(
            Error::TaskError {
                is_panic: false,
                is_cancel: true
            },
            Error::from(jh.await.expect_err("should be an error"))
        );

        let jh = handle.spawn(async { panic!() });
        assert_eq!(
            Error::TaskError {
                is_panic: true,
                is_cancel: false
            },
            Error::from(jh.await.expect_err("should be an error"))
        );
    }

    #[tokio::test]
    async fn test_plugin_mismatch() {
        let mut state = State::new();
        state.register(TestPlugin {
            skip: false,
            called: Arc::new(OnceLock::new()),
            call_mut: false,
        });

        let plugin = state.find_plugin(TestPlugin::name().path()).unwrap();
        let call = Call {
            item: plugin.clone(),
            cancel: CancellationToken::new(),
            handle: tokio::runtime::Handle::current(),
        };

        assert_eq!(
            Error::PluginMismatch,
            call.bind::<NotTestPlugin>()
                .expect_err("should have an error")
        );

        // Tests internal error handling
        let call = Call {
            item: plugin.clone(),
            cancel: CancellationToken::new(),
            handle: tokio::runtime::Handle::current(),
        };
        let mut bound = call.bind::<TestPlugin>().expect("should bind");
        bound.plugin().expect("should return a plugin");
        bound.plugin_mut().expect("should return a plugin");

        // Tests internal error handling
        let call = Call {
            item: plugin.clone(),
            cancel: CancellationToken::new(),
            handle: tokio::runtime::Handle::current(),
        };
        let mut bind = Bind::<NotTestPlugin> {
            call,
            _bound: std::marker::PhantomData,
        };
        assert_eq!(
            Error::PluginMismatch,
            bind.plugin().expect_err("should have an error")
        );
        assert_eq!(
            Error::PluginMismatch,
            bind.plugin_mut().expect_err("should have an error")
        );
    }

    #[tokio::test]
    async fn test_plugin_could_not_find_plugin() {
        let state = State::new();
        assert_eq!(
            Error::PluginNotFound,
            state
                .call("doesnt-exist")
                .await
                .expect_err("should return an error")
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_plugin_call() {
        let called = Arc::new(OnceLock::new());
        let mut state = State::init().await;
        state.register(TestPlugin {
            skip: false,
            called: called.clone(),
            call_mut: false,
        });

        let path = TestPlugin::name();
        let _ = state.call(path.path()).await.unwrap();
        assert!(called.get().unwrap());
        ()
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_plugin_call_work_mut() {
        let called = Arc::new(OnceLock::new());
        let mut state = State::init().await;
        state.register(TestPlugin {
            skip: false,
            called: called.clone(),
            call_mut: true,
        });

        let path = TestPlugin::name();
        let _ = state.call(path.path()).await.unwrap();
        assert!(called.get().unwrap());

        let plugin = state.find_plugin(path.path()).unwrap();
        let plugin = plugin.borrow::<TestPlugin>().unwrap();
        assert!(!plugin.call_mut);
        ()
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_plugin_call_skip() {
        let called = Arc::new(OnceLock::new());
        let mut state = State::init().await;
        state.register(TestPlugin {
            skip: true,
            called: called.clone(),
            call_mut: false,
        });

        let path = TestPlugin::name();
        assert_eq!(
            Error::PluginCallSkipped,
            state
                .call(path.path())
                .await
                .expect_err("should return an error")
        );
        ()
    }

    #[test]
    fn test_plugin_name() {
        let name = TestPlugin::name();
        assert_eq!(
            format!("reality/{}/tests/testplugin", env!("CARGO_PKG_VERSION")).as_str(),
            name.path().as_os_str()
        );
    }

    #[tokio::test]
    async fn test_plugin_call_by_path() {
        let called = Arc::new(OnceLock::new());
        let mut state = State::init().await;
        state.register(TestPlugin {
            skip: false,
            called: called.clone(),
            call_mut: false,
        });

        let _ = state.call("reality/0.1.0/tests/testplugin").await.unwrap();
        assert!(called.get().unwrap());
        ()
    }

    #[test]
    fn test_state_with_handle() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let called = Arc::new(OnceLock::new());
        let mut state = State::with(rt.handle().clone());
        state.register(TestPlugin {
            skip: false,
            called: called.clone(),
            call_mut: false,
        });

        rt.block_on(async move {
            let path = TestPlugin::name();
            let _ = state.call(path.path()).await.unwrap();
        });

        assert!(called.get().unwrap());
    }

    #[test]
    #[should_panic]
    fn test_state_panic_outside_tokio() {
        State::new();
    }

    #[derive(Clone, Serialize, Debug)]
    struct NotTestPlugin;

    impl Resource for NotTestPlugin {}
    impl Plugin for NotTestPlugin {
        fn call(_: Bind<Self>) -> Result<plugin::SpawnWork> {
            todo!()
        }
    }

    #[derive(Clone, Serialize)]
    struct TestPlugin {
        skip: bool,
        #[serde(skip)]
        called: Arc<OnceLock<bool>>,
        call_mut: bool,
    }

    impl Hash for TestPlugin {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.skip.hash(state);
        }
    }

    impl Resource for TestPlugin {}

    impl Plugin for TestPlugin {
        fn call(bind: Bind<Self>) -> Result<plugin::SpawnWork> {
            let plugin = bind.plugin()?;

            if plugin.skip {
                Err(Error::PluginCallSkipped)
            } else if plugin.call_mut {
                Ok(bind.work_mut(|test, _| {
                    let _ = test.called.set(true);
                    test.call_mut = false;
                    async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        Ok(())
                    }
                }))
            } else {
                Ok(bind.work(|test, _| {
                    let _ = test.called.set(true);
                    async move { Ok(()) }
                }))
            }
        }
    }
}
