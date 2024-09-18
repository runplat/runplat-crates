mod config;
pub use config::EventConfig;
pub use config::EngineConfig;

use reality::{Content, Repr, Resource, State, Uuid};
use crate::plugins::{Request, RequestArgs};
use super::Load;

/// Type-alias for a function that creates an environment
type CreateEnv = fn() -> State;

/// Struct containing tools for creating a new environment
pub struct Env {
    /// Label for this environment
    label: String,
    /// Engine config for this environment
    config: Option<EngineConfig>,
    /// Function for creating a new environment
    create_env: CreateEnv,
}

impl Env {
    /// Creates a new env repr
    #[inline]
    pub fn new(label: impl Into<String>, create_env: fn() -> State) -> Self {
        Self {
            label: label.into(),
            config: None,
            create_env,
        }
    }

    /// Sets the config for the environment
    #[inline]
    pub fn with_config(&mut self, config: EngineConfig) {
        self.config = Some(config);
    }

    /// Creates a new state
    #[inline]
    pub fn state(&self) -> State {
        (self.create_env)()
    }
}

impl Resource for Env {}
impl Repr for Env {}
impl Content for Env {
    fn state_uuid(&self) -> reality::uuid::Uuid {
        let mut crc = reality::content::crc().digest();
        crc.update(self.label.as_bytes());
        Uuid::from_u64_pair(crc.finalize(), 0)
    }
}

impl Default for Env {
    fn default() -> Self {
        Self {
            label: String::from("default"),
            config: None,
            create_env: default_env,
        }
    }
}

/// Creates an env w/ default set of plugins
pub fn default_env() -> State {
    let mut state = State::new();
    let _ = state.store_mut().put(Load::<Request>::by_toml()).commit();
    let _ = state
        .store_mut()
        .put(Load::<RequestArgs>::by_args())
        .commit();
    state
}
