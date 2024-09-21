mod env;
mod load;
mod operation;
pub use env::default_create_env;
pub use env::EngineConfig;
pub use env::EnvBuilder;
pub use env::Env;
pub use env::EventConfig;
pub use env::LoaderMetadata;
pub use env::BuildMetadata;
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
    use crate::{
        engine::{default_create_env, env::EnvBuilder, EventConfig, Operation},
        plugins::Request,
    };

    #[tokio::test]
    async fn test_env_loader_returns_error_when_no_valid_files_are_build() {
        let env = EnvBuilder::new("test_no_valid", default_create_env);
        env.build_env("tests/data", ".test").expect_err("should return an error because no valid files are found");
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_env_loader_can_load_identical_plugin_config() {
        let env = EnvBuilder::new("test_identical", default_create_env);
        env.build_env("tests/data", ".test").expect("should be able to build env");

        let loader = env
            .load_env(".test")
            .expect("should be able to load test env");

        let event = loader
            .create_event(&EventConfig {
                event: "test".to_string(),
                handler: None,
            })
            .unwrap();
        assert_eq!(
            "kioto/0.1.0/plugins/request/4849b2adfad5a5da",
            event.address().to_string()
        );

        let event = loader
            .create_event(&EventConfig {
                event: "test2".to_string(),
                handler: None,
            })
            .unwrap();
        assert_eq!(
            "kioto/0.1.0/plugins/request/fb51ee142f39ae12",
            event.address().to_string()
        );
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_env_loader_test_operation() {
        // Builder to build and load an env
        let env = EnvBuilder::default_env("test_operation");
        
        // Test building the env before trying to load it
        env.build_env("tests/data", ".test").unwrap();
    
        // Load a new environment
        let env = env
            .load_env(".test")
            .expect("should be able to load test env");

        let event = env
            .create_event(&EventConfig {
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
