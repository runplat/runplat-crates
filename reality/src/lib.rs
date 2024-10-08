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

mod content_utils;
pub use content_utils::BincodeContent;
pub use content_utils::NilContent;
pub use content_utils::RandomContent;

/// Re-export derive macro
pub use runplat_macros::Plugin;
pub use runplat_macros::Repr;
pub use runplat_macros::Resource;

/// Re-export runir since it will be required for extending reality
pub use runir;
pub use runir::content;
pub use runir::*;

/// Re-export semver::Version
pub use semver::Version;

/// Re-export uuid::Uuid
pub use uuid;
pub use uuid::Uuid;

/// Type-alias for spawning work
pub type CallResult = Result<plugin::Work>;

/// Type-alias for this crates main result type
pub type Result<T> = std::result::Result<T, Error>;

/// Enum of error variants produced by this library
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Error {
    /// Error when a join handle can not run to completion, analagous to tokio::runtime::JoinError
    TaskError { is_panic: bool, is_cancel: bool },
    /// Error when a join handle can not run to completion, analagous to tokio::runtime::JoinError
    IOError { message: String },
    /// Error when a serialization error occurs
    SerializationError {
        message: String,
        format: SerializationFormat,
    },
    /// Error returned when a plugin could not be loaded from a path
    LoadPluginError,
    /// Error returned when a `Name` could not be parsed
    IncompletePluginName,
    /// Error returned when previous request data exists for a handle
    PreviousUnhandledRequest,
    /// Error returned when trying to write request data, expecting the entry to
    /// be empty, but replacing an existing entry
    WriteRequestRaceCondition,
    /// Error when a plugin cannot be found in the current state
    PluginNotFound,
    /// Error returned when casting a dynamic pointer to a plugin
    PluginMismatch,
    /// Error returned when the trying to add a handler to a plugin event
    /// and the the handler's target does not match the type of backing the
    /// event
    PluginHandlerTargetMismatch,
    /// Error returned when a plugin handlercall is skipped by the plugin
    PluginHandlerCallSkipped,
    /// Error returned when a plugin call is cancelled
    PluginCallCancelled,
    /// Error returned when a plugin call is skipped by the plugin
    PluginCallSkipped,
    /// Custom error returned by the implementation of the plugin
    PluginCallError {
        /// Name of the plugin where the error occured
        name: plugin::Name,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SerializationFormat {
    Toml,
    Json,
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Self::TaskError {
            is_panic: e.is_panic(),
            is_cancel: e.is_cancelled(),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IOError {
            message: e.to_string(),
        }
    }
}

impl From<toml::ser::Error> for Error {
    fn from(value: toml::ser::Error) -> Self {
        Error::SerializationError {
            message: value.to_string(),
            format: SerializationFormat::Toml,
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::SerializationError {
            message: value.to_string(),
            format: SerializationFormat::Toml,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Error::SerializationError {
            message: value.to_string(),
            format: SerializationFormat::Json,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::*;
    use plugin::{Bind, Call, Handler, Plugin, State, Work};
    use repr::Labels;
    use runir::Resource;
    use runplat_macros::Plugin;
    use semver::Version;
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

    impl Content for TomlPlugin {
        fn state_uuid(&self) -> uuid::Uuid {
            BincodeContent::new(self).unwrap().state_uuid()
        }
    }

    impl Plugin for TomlPlugin {
        fn call(_: Bind<Self>) -> Result<plugin::Work> {
            Err(Error::PluginCallSkipped)
        }

        fn version() -> semver::Version {
            semver::Version::new(0, 1, 0)
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
            .load_by_toml::<TomlPlugin>(
                &toml.expect("should be able to serialize"),
                Labels::default(),
            )
            .expect("should be able to load");

        state
            .addresses()
            .pop()
            .expect("should have loaded the plugin");
    }

    #[tokio::test]
    async fn test_plugin_replacement() {
        let mut state = State::new();
        state.load(
            TestPlugin {
                skip: false,
                called: Arc::new(OnceLock::new()),
                call_mut: false,
            },
            Labels::default(),
        );

        state.load(
            TestPlugin {
                skip: true,
                called: Arc::new(OnceLock::new()),
                call_mut: false,
            },
            Labels::default(),
        );
        let addresses = state.addresses();
        addresses.get(0).expect("should have address");
        addresses.get(1).expect("should have address");
        assert_eq!(2, state.addresses().len());
    }

    #[tokio::test]
    async fn test_plugin_work_cancel() {
        let mut state = State::new();
        state.load(
            TestPlugin {
                skip: false,
                called: Arc::new(OnceLock::new()),
                call_mut: false,
            },
            Labels::default(),
        );

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
        state.load(
            TestPlugin {
                skip: false,
                called: Arc::new(OnceLock::new()),
                call_mut: false,
            },
            Labels::default(),
        );

        let plugin = state.find_plugin(TestPlugin::name().path()).unwrap();
        let call = Call {
            state: state.clone(),
            item: plugin.clone(),
            fork_fn: TestPlugin::fork,
            cancel: CancellationToken::new(),
            runtime: tokio::runtime::Handle::current(),
            handler: None,
        };

        assert_eq!(
            Error::PluginMismatch,
            call.bind::<NotTestPlugin>()
                .expect_err("should have an error")
        );

        // Tests internal error handling
        let call = Call {
            state: state.clone(),
            item: plugin.clone(),
            fork_fn: TestPlugin::fork,
            cancel: CancellationToken::new(),
            runtime: tokio::runtime::Handle::current(),
            handler: None,
        };
        let mut bound = call.bind::<TestPlugin>().expect("should bind");
        bound.receiver().expect("should return a plugin");
        bound.update().expect("should return a plugin");

        // Tests internal error handling
        let call = Call {
            state: state.clone(),
            item: plugin.clone(),
            fork_fn: TestPlugin::fork,
            cancel: CancellationToken::new(),
            runtime: tokio::runtime::Handle::current(),
            handler: None,
        };
        let mut bind = Bind::<NotTestPlugin> {
            call,
            receiver: None,
            _bound: std::marker::PhantomData,
        };
        assert_eq!(
            Error::PluginMismatch,
            bind.receiver().expect_err("should have an error")
        );
        assert_eq!(
            Error::PluginMismatch,
            bind.update().expect_err("should have an error")
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
        state.load(
            TestPlugin {
                skip: false,
                called: called.clone(),
                call_mut: false,
            },
            Labels::default(),
        );

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
        state.load(
            TestPlugin {
                skip: false,
                called: called.clone(),
                call_mut: true,
            },
            Labels::default(),
        );

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
        state.load(
            TestPlugin {
                skip: true,
                called: called.clone(),
                call_mut: false,
            },
            Labels::default(),
        );

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
        state.load(
            TestPlugin {
                skip: false,
                called: called.clone(),
                call_mut: false,
            },
            Labels::default(),
        );

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
        state.load(
            TestPlugin {
                skip: false,
                called: called.clone(),
                call_mut: false,
            },
            Labels::default(),
        );

        rt.block_on(async move {
            let path = TestPlugin::name();
            let _ = state.call(path.path()).await.unwrap();
        });

        assert!(called.get().unwrap());
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_state_with_event_handler() {
        let called = Arc::new(OnceLock::new());
        let mut state = State::init().await;
        state.load(
            TestPlugin {
                skip: false,
                called: called.clone(),
                call_mut: false,
            },
            Labels::default(),
        );
        let handler = state.load(TestPluginHandler { test_plugin: None }, Labels::default());

        let mut event = state.event("reality/0.1.0/tests/testplugin").unwrap();
        event.with_handler::<TestPluginHandler>(handler).unwrap();
        event.start().await.unwrap();

        let event = state
            .event("reality/0.1.0/tests/testpluginhandler")
            .unwrap();
        let handler = event.call.item.borrow::<TestPluginHandler>().unwrap();
        assert!(handler.test_plugin.is_some());
    }

    #[test]
    #[should_panic]
    fn test_state_panic_outside_tokio() {
        State::new();
    }

    #[derive(Clone, Serialize, Debug)]
    pub struct NotTestPlugin;

    impl Resource for NotTestPlugin {}
    impl Plugin for NotTestPlugin {
        fn call(_: Bind<Self>) -> Result<plugin::Work> {
            todo!()
        }

        fn version() -> semver::Version {
            Version::new(0, 1, 0)
        }
    }

    impl Content for NotTestPlugin {
        fn state_uuid(&self) -> uuid::Uuid {
            BincodeContent::new(self).unwrap().state_uuid()
        }
    }

    #[derive(Clone, Serialize)]
    pub struct TestPluginHandler {
        test_plugin: Option<TestPlugin>,
    }

    impl Resource for TestPluginHandler {}
    impl Content for TestPluginHandler {
        fn state_uuid(&self) -> uuid::Uuid {
            BincodeContent::new(self).unwrap().state_uuid()
        }
    }
    impl Plugin for TestPluginHandler {
        fn call(bind: Bind<Self>) -> Result<plugin::Work> {
            bind.work(|_, _| async { Ok(()) })
        }
        fn version() -> Version {
            Version::new(0, 1, 0)
        }
    }

    impl Handler for TestPluginHandler {
        type Target = TestPlugin;

        fn handle(other: Bind<Self::Target>, mut handler: Bind<Self>) -> Result<()> {
            let handler = handler.update()?;
            let target = other.receiver()?.clone();
            handler.test_plugin = Some(target);
            Ok(())
        }
    }

    #[derive(Clone, Serialize)]
    pub struct TestPlugin {
        skip: bool,
        #[serde(skip)]
        called: Arc<OnceLock<bool>>,
        call_mut: bool,
    }

    impl Resource for TestPlugin {}

    impl Plugin for TestPlugin {
        fn call(bind: Bind<Self>) -> Result<Work> {
            let plugin = bind.receiver()?;

            if plugin.skip {
                Err(Error::PluginCallSkipped)
            } else if plugin.call_mut {
                bind.work_mut(|test, _| {
                    let _ = test.called.set(true);
                    test.call_mut = false;
                    async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        Ok(())
                    }
                })
            } else {
                bind.work(|test, _| {
                    let _ = test.called.set(true);
                    async move { Ok(()) }
                })
            }
        }

        fn version() -> Version {
            Version::new(0, 1, 0)
        }
    }

    impl Content for TestPlugin {
        fn state_uuid(&self) -> uuid::Uuid {
            BincodeContent::new(self).unwrap().state_uuid()
        }
    }

    #[derive(Plugin, Serialize)]
    #[reality(
        call = call_test_derive,
        content_with = test_content_with,
    )]
    struct TestDerive;

    fn call_test_derive(bind: Bind<TestDerive>) -> CallResult {
        bind.work(|_, _| async { Ok(()) })
    }

    fn test_content_with(_: &TestDerive) -> Uuid {
        Uuid::new_v4()
    }

    #[tokio::test]
    async fn test_test_derive() {
        let mut state = State::new();
        let _ = state.load(TestDerive, Labels::default());
    }
}
