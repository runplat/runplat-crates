use std::future::Future;
use std::pin::pin;
use tokio_util::sync::CancellationToken;
use crate::Error;

pub struct Work {
    /// Running task
    pub(super) task: tokio::task::JoinHandle<crate::Result<()>>,
    /// Cancellation token for this work
    pub(super) cancel: CancellationToken,
}

impl Future for Work {
    type Output = crate::Result<()>;
    
    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.cancel.is_cancelled() {
            self.task.abort();
            return std::task::Poll::Ready(Err(Error::PluginAborted));
        }
        let task = &mut self.as_mut().task;
        let pinned = pin!(task);
        match pinned.poll(cx) {
            std::task::Poll::Ready(r) => {
                std::task::Poll::Ready(r?)
            },
            std::task::Poll::Pending => {
                std::task::Poll::Pending
            },
        }
    }
}
