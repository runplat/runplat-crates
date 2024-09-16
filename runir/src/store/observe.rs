use crate::Resource;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
    pin::Pin,
    sync::{Arc, Condvar, Mutex, RwLock},
    time::{Duration, Instant},
};

/// Struct for observing an item
#[derive(Clone)]
pub struct ObservationEvent {
    /// Sync context
    pub(crate) sync: SyncContext,
}

/// Type-alias for a synchronization context
type SyncContext = Arc<(Mutex<ObvservationState>, Condvar)>;

/// Observation state shared by observer and observed
pub(crate) struct ObvservationState {
    pub accessed: bool,
    start: Option<Instant>,
    timeout: Option<Duration>,
}

impl ObservationEvent {
    /// Creates a new observation
    pub(crate) fn new() -> Self {
        Self {
            sync: Arc::new((
                Mutex::new(ObvservationState {
                    accessed: false,
                    start: None,
                    timeout: None,
                }),
                Condvar::new(),
            )),
        }
    }

    /// Sets the timeout setting for the observation state
    pub(crate) fn timeout(&mut self, timeout: impl Into<Duration>) -> &mut Self {
        let mut v = match self.sync.0.lock() {
            Ok(v) => v,
            Err(e) => e.into_inner(),
        };

        v.timeout = Some(timeout.into());
        drop(v);
        self
    }

    /// Waits for an observation to complete,
    ///
    /// Returns true after a change has occurred
    pub fn wait(self) -> bool {
        let sync = &*self.sync;
        let mut guard = match sync.0.lock() {
            Ok(g) => g,
            Err(err) => err.into_inner(),
        };

        guard.start = Some(Instant::now());

        let g = if let Some(timeout) = guard.timeout.take() {
            match sync.1.wait_timeout_while(guard, timeout, |o| !o.accessed) {
                Ok(g) => g,
                Err(err) => err.into_inner(),
            }.0
        } else {
            match sync.1.wait_while(guard, |o| !o.accessed) {
                Ok(g) => g,
                Err(err) => err.into_inner(),
            }
        };

        g.accessed
    }
}