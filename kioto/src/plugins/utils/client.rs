use std::{future::Future, pin::Pin, process::Output};

use crate::plugins::{Process, Request};
use hyper::body::Incoming;
use reality::{plugin::{Handler, MessageData}, Content, Plugin, Resource, Uuid, Version};
use super::with_cancel;

/// Type-alias for a process client
pub type ProcessClient = Client<Output>;

/// Type-alias for an http request client
pub type HttpRequestClient = Client<IncomingResponse>;

/// Type-alias for an incoming response from an http request
pub type IncomingResponse = hyper::Response<Incoming>;

/// Type-alias for a client return function
pub type ReturnFn<R> = Box<
    dyn Fn(R) -> Pin<Box<dyn Future<Output = reality::Result<MessageData>> + Send>> + Send + Sync + 'static,
>;

/// Utility for constructing a "client" type from an environment
pub struct Client<R> {
    /// Called after the handler has a result set, output is sent to the handler's commit id
    returns: ReturnFn<R>,
    /// Result of the target plugin this client is attached to
    result: Option<R>,
}

impl<R> Client<R> {
    /// Creates a new client with function
    #[inline]
    pub fn new(
        next: impl Fn(R) -> Pin<Box<dyn Future<Output = reality::Result<MessageData>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self { returns: Box::new(next), result: None }
    }
}

impl<R: Send + Sync + 'static> Plugin for Client<R> {
    fn call(bind: reality::plugin::Bind<Self>) -> reality::CallResult {
        bind.defer(|mut binding, ct| async move {
            if let Some(r) = binding.update()?.result.take() {
                let returns = with_cancel(ct)
                    .run((binding.receiver()?.returns)(r))
                    .await??;
                let reply_to = binding.item().commit();
                binding.broker().send(reply_to, returns)
            } else {
                Err(reality::Error::PluginCallSkipped)
            }
        })
    }

    fn version() -> reality::Version {
        Version::parse(env!("CARGO_PKG_VERSION")).expect("should be successful because cargo would not compile")
    }
}

impl Handler for Client<IncomingResponse> {
    type Target = Request;

    fn handle(
        mut other: reality::plugin::Bind<Self::Target>,
        mut handler: reality::plugin::Bind<Self>,
    ) -> reality::Result<()> {
        handler.update()?.result = other.update()?.take_response();
        Ok(())
    }
}

impl Handler for Client<std::process::Output> {
    type Target = Process;

    fn handle(
        mut other: reality::plugin::Bind<Self::Target>,
        mut handler: reality::plugin::Bind<Self>,
    ) -> reality::Result<()> {
        handler.update()?.result = other.update()?.take_output();
        Ok(())
    }
}

impl<R: Send + Sync + 'static> Resource for Client<R> {}
impl<R> Content for Client<R> {
    fn state_uuid(&self) -> reality::Uuid {
        Uuid::new_v4()
    }
}
