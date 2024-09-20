mod env;
mod load;
mod operation;
pub use env::default_env;
pub use env::EngineConfig;
pub use env::Env;
pub use env::EnvLoader;
pub use env::EventConfig;
pub use env::Metadata;
pub use load::Load;
pub use load::LoadBy;
pub use load::LoadInput;
pub use operation::Operation;

use reality::plugin::Event;
use reality::State;

/// An engine manages a collection of events and plugin resources
pub struct Engine {
    /// Engine state which stores plugin resources
    state: State,
    /// Collection of events created by this engine
    events: Vec<Event>,
}

impl Engine {
    /// Creates an engine with state
    #[inline]
    pub fn with(state: reality::State) -> Self {
        Engine {
            state,
            events: vec![],
        }
    }

    /// Creates and pushes a plugin event onto the engine
    #[inline]
    pub fn push(&mut self, event: Event) -> reality::Result<()> {
        self.events.push(event);
        Ok(())
    }

    /// Returns an event pushed on to this engine
    #[inline]
    pub fn event(&self, index: usize) -> Option<&Event> {
        self.events.get(index)
    }

    /// Returns a reference to the engine's state
    #[inline]
    pub fn state(&self) -> &State {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use reality::Plugin;
    use std::path::PathBuf;

    use crate::{
        engine::{EventConfig, Operation},
        plugins::Request,
        test_env,
    };
    test_env!(test);

    #[tokio::test]
    async fn test_env_loader_can_load_from_filesystem() {
        let test_dir = PathBuf::from(".test/test");
        let test_source = PathBuf::from("tests/data");
        if test_dir.exists() {
            std::fs::remove_dir_all(&test_dir).unwrap();
        }
        std::fs::create_dir_all(&test_dir).unwrap();

        let test_config_src = test_source.join("test").join("config.toml");
        let test_config_dest = test_dir.join("config.toml");
        std::fs::copy(test_config_src, test_config_dest).unwrap();
        let test_config_plugin_test = test_source
            .join("test")
            .join("etc")
            .join(Request::name().path())
            .join("test.toml");
        let test_config_plugin_test_dest = test_dir.join("etc").join(Request::name().path());
        std::fs::create_dir_all(&test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test_dest = test_config_plugin_test_dest.join("test.toml");
        std::fs::copy(test_config_plugin_test, test_config_plugin_test_dest).unwrap();

        let test_config_plugin_test = test_source
            .join("test")
            .join("etc")
            .join(Request::name().path())
            .join("test2.toml");
        let test_config_plugin_test_dest = test_dir.join("etc").join(Request::name().path());
        std::fs::create_dir_all(&test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test_dest = test_config_plugin_test_dest.join("test2.toml");
        std::fs::copy(test_config_plugin_test, test_config_plugin_test_dest).unwrap();
        let env = test::env();
        let loader = env
            .env_loader(".test")
            .expect("should be able to load test env");

        let event = loader
            .get_event(&EventConfig {
                event: "test".to_string(),
                handler: None,
            })
            .unwrap();
        assert_eq!(
            "kioto/0.1.0/plugins/request/b937ab23c51e66ac",
            event.address().to_string()
        );

        let event = loader
            .get_event(&EventConfig {
                event: "test2".to_string(),
                handler: None,
            })
            .unwrap();
        assert_eq!(
            "kioto/0.1.0/plugins/request/48b5b448d8d9cdce",
            event.address().to_string()
        );
    }

    test_env!(test_operation);

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_env_loader_test_operation() {
        let test_dir = PathBuf::from(".test/test_operation");
        let test_source = PathBuf::from("tests/data");
        if test_dir.exists() {
            std::fs::remove_dir_all(&test_dir).unwrap();
        }
        std::fs::create_dir_all(&test_dir).unwrap();

        let test_config_src = test_source.join("test_operation").join("config.toml");
        let test_config_dest = test_dir.join("config.toml");
        std::fs::copy(test_config_src, test_config_dest).unwrap();
        let test_config_plugin_test = test_source
            .join("test_operation")
            .join("etc")
            .join(Request::name().path())
            .join("test.toml");
        let test_config_plugin_test_dest = test_dir.join("etc").join(Request::name().path());
        std::fs::create_dir_all(&test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test_dest = test_config_plugin_test_dest.join("test.toml");
        std::fs::copy(test_config_plugin_test, test_config_plugin_test_dest).unwrap();

        let test_config_plugin_test = test_source
            .join("test_operation")
            .join("etc")
            .join(Request::name().path())
            .join("test2.toml");
        let test_config_plugin_test_dest = test_dir.join("etc").join(Request::name().path());
        std::fs::create_dir_all(&test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test_dest = test_config_plugin_test_dest.join("test2.toml");
        std::fs::copy(test_config_plugin_test, test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test = test_source
            .join("test_operation")
            .join("etc")
            .join(Operation::name().path())
            .join("run_tests.toml");
        let test_config_plugin_test_dest = test_dir.join("etc").join(Operation::name().path());
        std::fs::create_dir_all(&test_config_plugin_test_dest).unwrap();
        let test_config_plugin_test_dest = test_config_plugin_test_dest.join("run_tests.toml");
        std::fs::copy(test_config_plugin_test, test_config_plugin_test_dest).unwrap();

        let test_config_src = test_source.join("test_operation").join("config.toml");
        let test_config_dest = test_dir.join("config.toml");
        std::fs::copy(test_config_src, test_config_dest).unwrap();

        let env = test_operation::env();
        let loader = env
            .env_loader(".test")
            .expect("should be able to load test env");

        let event = loader
            .get_event(&EventConfig {
                event: "run_tests".to_string(),
                handler: None,
            })
            .unwrap();
        let event_clone = event.clone();
        event.start().await.unwrap();

        let engine = event_clone
            .item()
            .clone()
            .borrow_mut::<Operation>()
            .unwrap()
            .take_engine()
            .unwrap();

        let event = engine.event(0).unwrap();
        let resp = event
            .item()
            .clone()
            .borrow_mut::<Request>()
            .unwrap()
            .take_response()
            .unwrap();
        assert!(resp.status().is_success());
    }
}
