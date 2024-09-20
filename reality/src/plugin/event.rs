use runir::store::Item;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use super::{thunk::HandlerThunk, Address, Call, Handler, Thunk};
use crate::{Error, Result};

/// Intermediary for calling a plugin
#[derive(Clone)]
pub struct Event {
    /// Resolved plugin address that created this event
    pub(crate) address: Address,
    /// Call state
    pub(crate) call: Call,
    /// Plugin thunk
    pub(crate) thunk: Thunk,
    /// Plugin handler thunk
    pub(crate) handler: Option<Thunk>,
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
                handler: self.handler.clone(),
            },
            cancel,
        )
    }

    /// Sets the handler on this event
    /// 
    /// Returns an error if the handler's associated Target type does not match
    /// the current event's plugin type
    #[inline]
    pub fn with_handler<H: Handler>(&mut self) -> Result<&mut Self> {
        if self.call.item.is_type::<H::Target>() {
            self.handler = Some(Thunk::handler::<H>());
            Ok(self)
        } else {
            Err(Error::PluginMismatch)
        }
    }

    /// Sets the handler on this event w/o generic typing
    /// 
    /// Returns an error if the handler's associated Target type does not match the current
    /// event's plugin type
    #[inline]
    pub fn set_handler(&mut self, handler: &HandlerThunk) -> Result<&mut Self> {
        if self.call.item.matches_type(handler.target_type()) {
            self.handler = Some(handler.thunk());
            Ok(self)
        } else {
            Err(Error::PluginMismatch)
        }
    }


    /// Consumes and starts the event
    #[inline]
    pub async fn start(self) -> Result<()> {
        if let Some(handler) = self.handler {
            handler.exec(self.call).await
        } else {
            debug!(address = self.address().to_string(), "event_start");
            self.thunk.exec(self.call).await
        }
    }

    /// Returns the resource for this event
    #[inline]
    pub fn item(&self) -> &Item {
        &self.call.item
    }
}
