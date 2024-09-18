use tokio_util::sync::CancellationToken;
use tracing::debug;

use super::{Address, Call, Thunk};
use crate::Result;

/// Intermediary for calling a plugin
pub struct Event {
    /// Resolved plugin address that created this event
    pub(crate) address: Address,
    /// Call state
    pub(crate) call: Call,
    /// Plugin thunk
    pub(crate) thunk: Thunk,
}

impl Event {
    /// Returns the address for this event
    #[inline]
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Forks the event and returns the forked event and associated cancellation token
    ///
    /// Forking the event preserves the main cancellation token for this event so that the forked event can be cancelled,
    /// without deterioating the event source
    #[inline]
    pub fn fork(&self) -> (Self, CancellationToken) {
        let forked = self.call.fork();
        let cancel = forked.cancel.clone();
        (
            Self {
                address: self.address.clone(),
                call: forked,
                thunk: self.thunk.clone(),
            },
            cancel,
        )
    }

    /// Consumes and starts the event
    #[inline]
    pub async fn start(self) -> Result<()> {
        debug!(address = self.address().to_string(), "event_start");
        let work = self.thunk.exec(self.call).await?;
        work.await
    }
}
