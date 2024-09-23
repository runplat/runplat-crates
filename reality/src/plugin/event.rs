use std::sync::Arc;

use super::{thunk::HandlerThunk, Address, Call, Handler, MessageData, Thunk};
use crate::{Error, Result};
use runir::{repr::Labels, store::Item};
use tokio_util::sync::CancellationToken;
use tracing::debug;

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
    /// Labels
    pub(crate) labels: Option<Arc<Labels>>,
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
                labels: self.labels.clone(),
            },
            cancel,
        )
    }

    /// Sets the handler on this event
    ///
    /// Returns an error if the handler's associated Target type does not match
    /// the current event's plugin type
    #[inline]
    pub fn with_handler<H: Handler>(&mut self, address: Address) -> Result<&mut Self> {
        if self.call.item.is_type::<H::Target>() {
            self.call.set_handler(address);
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
    pub fn set_handler(&mut self, address: Address, handler: &HandlerThunk) -> Result<&mut Self> {
        if self.call.item.matches_type(handler.target_type()) {
            self.call.set_handler(address);
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

    /// Consumes and starts the event, if the event was assigned a handler, returns
    /// any messages received by the handler
    #[inline]
    pub async fn returns(self) -> Result<MessageData> {
        if let Some(handler) = self.handler {
            let handler_info = self.call.handler().cloned();
            let broker = self.call.state.broker().clone();
            handler.exec(self.call).await?;
            let returns = handler_info
                .map(|h| broker.receive(h.commit()))
                .unwrap_or_default();
            Ok(returns)
        } else {
            debug!(address = self.address().to_string(), "event_start");
            self.thunk.exec(self.call).await?;
            Ok(MessageData::Empty)
        }
    }

    /// Returns the resource for this event
    #[inline]
    pub fn item(&self) -> &Item {
        &self.call.item
    }

    /// Returns the value of a label
    #[inline]
    pub fn label(&self, label: &str) -> Option<&str> {
        self.labels
            .as_ref()
            .and_then(|l| l.get(label).map(|l| l.as_str()))
    }
}
