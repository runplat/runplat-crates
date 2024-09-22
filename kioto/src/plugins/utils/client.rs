use std::{future::Future, pin::Pin};

use crate::plugins::Request;
use hyper::body::Incoming;
use reality::{plugin::Handler, Content, Plugin, Resource, Uuid};

use super::with_cancel;

pub type HttpRequestClient = Client<IncomingResponse>;

pub type IncomingResponse = hyper::Response<Incoming>;

pub type ClientFn<R> = Box<
    dyn Fn(R) -> Pin<Box<dyn Future<Output = reality::Result<()>> + Send>> + Send + Sync + 'static,
>;

/// Utility for constructing a "client" type from an environment
pub struct Client<R> {
    next: ClientFn<R>,
    result: Option<R>,
}

impl<R> Client<R> {
    /// Creates a new client from function
    #[inline]
    pub fn new(
        next: impl Fn(R) -> Pin<Box<dyn Future<Output = reality::Result<()>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self { next: Box::new(next), result: None }
    }
}

impl Plugin for Client<IncomingResponse> {
    fn call(bind: reality::plugin::Bind<Self>) -> reality::CallResult {
        bind.defer(|mut binding, ct| async move {
            if let Some(r) = binding.update()?.result.take() {
                with_cancel(ct)
                    .run((binding.receiver()?.next)(r), |r| r)
                    .await
            } else {
                Err(reality::Error::PluginCallSkipped)
            }
        })
    }

    fn version() -> reality::Version {
        reality::Version::new(0, 1, 0)
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

impl<R: Send + Sync + 'static> Resource for Client<R> {}
impl<R> Content for Client<R> {
    fn state_uuid(&self) -> reality::Uuid {
        Uuid::new_v4()
    }
}
