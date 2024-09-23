use std::{
    sync::{
        atomic::AtomicU64,
        Arc, Condvar, Mutex, MutexGuard,
    }, time::{Duration, Instant}
};
use crate::Resource;
use super::Item;

/// Observable item wrapper to enable "Observer" pattern using items.
/// 
/// Bound to a type in order to 
pub struct Observable {
    /// Inner item being observed
    pub(crate) inner: Item,
    /// Event state
    pub(crate) event: ObservationEvent,
}

impl Observable {
    /// Returns an observation event that can be used to receive changes
    #[inline]
    pub fn event(&self) -> ObservationEvent {
        self.event.clone()
    }

    /// Returns a mutable pointer to the inner type
    #[inline]
    pub fn borrow_mut<T: Resource>(&mut self) -> Option<&mut T> {
        self.inner.borrow_mut()
    }

    /// Returns a reference to the inner type
    #[inline]
    pub fn borrow<T: Resource>(&self) -> Option<&T> {
        self.inner.borrow()
    }

    /// Notifies listeners that the event has started
    #[inline]
    pub fn notify_start(&mut self) {
        let (mut state, v) = self.notify();
        state.start = Some(Instant::now());
        v.notify_all();
        drop(state);
    }

    /// Notifies a change has occurred and updates the message
    #[inline]
    pub fn notify_change_with_message(&mut self, message: impl Into<String>) {
        let (mut state, v) = self.notify();
        state.message = message.into();
        v.notify_all();
        drop(state);
    }

    /// Notifies a change has occurred and updates progress
    #[inline]
    pub fn notify_change_with_progress(&mut self, progress: u64) {
        let (mut state, v) = self.notify();
        state.progress = progress;
        v.notify_all();
        drop(state);
    }

    /// Notifies a change has occurred
    #[inline]
    pub fn notify_change(&mut self) {
        let (state, v) = self.notify();
        v.notify_all();
        drop(state);
    }

    /// Begins a notification and increments the version
    #[inline]
    fn notify(&mut self) -> (MutexGuard<'_, ObvservationState>, &Condvar) {
        let sync = &*self.event.sync;
        let g = match sync.0.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        g.version
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        (g, &sync.1)
    }
}

/// Struct for observing an item
#[derive(Clone)]
pub struct ObservationEvent {
    /// Sync context
    pub(crate) sync: SyncContext,
    /// If set, will be the max time any wait operation will take
    timeout: Option<Duration>,
    /// Last version observed when this event last waited on the observed
    last_version: u64,
}

/// Current observed state
pub struct CurrentState {
    /// Version counter of the state when the event returned from wait
    pub version: u64,
    /// Current progress set in state when returned
    pub progress: u64,
    /// Current message set in state when returned
    pub message: String,
    /// Elapased duration of the current state if a start time was set, if not this will be set to 0
    pub elapsed: Duration,
}

/// Type-alias for a synchronization context
type SyncContext = Arc<(Mutex<ObvservationState>, Condvar)>;

/// Observation state shared by observer and observed
pub(crate) struct ObvservationState {
    /// Version counter which signals the condvar a change has occurred
    version: AtomicU64,
    /// Progress integer
    /// 
    /// **Note**: Since floats can be a bit undeterministic, an integer is used to represent "progress", but 
    /// no definition of what progress interval is in use is assumed
    progress: u64,
    /// Message that can be set by the observed and read by the observer
    message: String,
    /// Start time according to the observed
    /// 
    /// **Note**: This is optional and not required by the observed to set
    start: Option<Instant>,
}

impl ObservationEvent {
    /// Creates a new observation event
    /// 
    /// This will create an initial synchronization state
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            sync: Arc::new((
                Mutex::new(ObvservationState {
                    version: AtomicU64::new(0),
                    progress: 0,
                    message: String::new(),
                    start: None,
                }),
                Condvar::new(),
            )),
            timeout: None,
            last_version: 0,
        }
    }

    /// Sets the timeout setting for the observation state
    #[inline]
    pub fn timeout(&mut self, timeout: impl Into<Duration>) {
        self.timeout = Some(timeout.into());
    }

    /// Waits for a modification from the observed item
    ///
    /// Returns true after a change has occurred
    pub fn wait(&mut self) -> CurrentState {
        let sync = &*self.sync;
        let guard = match sync.0.lock() {
            Ok(g) => g,
            Err(err) => err.into_inner(),
        };

        let last_state = self.last_version;
        let g = if let Some(timeout) = self.timeout.as_ref() {
            match sync.1.wait_timeout_while(guard, *timeout, |o| {
                last_state == o.version.load(std::sync::atomic::Ordering::Relaxed)
            }) {
                Ok(g) => g,
                Err(err) => err.into_inner(),
            }
            .0
        } else {
            match sync.1.wait_while(guard, |o| {
                last_state == o.version.load(std::sync::atomic::Ordering::Relaxed)
            }) {
                Ok(g) => g,
                Err(err) => err.into_inner(),
            }
        };

        self.last_version = g.version.load(std::sync::atomic::Ordering::Relaxed);

        CurrentState {
            version: self.last_version,
            progress: g.progress,
            message: g.message.clone(),
            elapsed: g.start.map(|s| s.elapsed()).unwrap_or(Duration::default()),
        }
    }
}
