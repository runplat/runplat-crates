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

/// Type-alias for this crates main result type
pub type Result<T> = std::result::Result<T, Error>;

/// Enum of error variants produced by this library
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Error {
    JoinError { is_panic: bool, is_cancel: bool },
    CouldNotFindPlugin,
    PluginMismatch,
    PluginAborted,
    PluginCallSkipped,
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::JoinError {
            is_panic: e.is_panic(),
            is_cancel: e.is_cancelled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use plugin::{Bind, Plugin, State};
    use runir::Resource;
    use std::{
        env, hash::Hash, sync::{Arc, OnceLock}
    };

    #[tokio::test]
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

    #[tokio::test]
    async fn test_plugin_call_work_mut() {
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

    #[derive(Clone)]
    struct TestPlugin {
        skip: bool,
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
                    test.call_mut = false;
                    async move { Ok(()) }
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
