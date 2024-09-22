mod client;
pub use client::Client;
pub use client::HttpRequestClient;

use std::future::Future;
use clap::{Args, Subcommand};
use serde::Serialize;
use tokio::select;
use tokio_util::sync::CancellationToken;

/// Re-export of Loader Metadata type, which can be used as a field
/// to retrieve metadata information from when the plugin loaded
pub use crate::engine::LoaderMetadata;

/// Re-export of Build Metadata type
pub use crate::engine::BuildMetadata;

/// Re-export of TemplateMap type
pub use crate::engine::TemplateMap;

/// Re-export of TemplateField type
pub use crate::engine::TemplateField;

/// Common plugin commands to execute
#[derive(Serialize, Subcommand, Default)]
pub enum PluginCommands {
    /// Run the plugin's default call behavior
    #[default]
    Run,
    /// Exports the plugin configuration to the current environment
    Export(ExportArgs),
}

#[derive(Args, Serialize)]
pub struct ExportArgs {
    /// Name for the exported plugin settings
    pub name: String,
}

pub fn with_cancel(token: CancellationToken) -> TaskCancelWrapper {
    token.into()
}

pub struct TaskCancelWrapper {
    cancel: CancellationToken,
}

impl From<CancellationToken> for TaskCancelWrapper {
    fn from(value: CancellationToken) -> Self {
        Self { cancel: value }
    }
}

impl TaskCancelWrapper {
    pub async fn run<F, O>(
        self,
        fut: F,
        on_complete: impl FnOnce(O) -> reality::Result<()>,
    ) -> reality::Result<()>
    where
        F: Future<Output = O>,
    {
        select! {
            o = fut => {
                on_complete(o)
            },
            _ = self.cancel.cancelled() => {
                Err(reality::Error::PluginCallCancelled.into())
            }
        }
    }

    /// Run a future to completion w/ a cancellation token and on_completion
    /// execute a callback that transforms the output of the future to some return
    /// type
    pub async fn returns<F, O, R>(
        self,
        fut: F,
        on_complete: impl FnOnce(O) -> reality::Result<R>,
    ) -> reality::Result<R>
    where
        F: Future<Output = O>,
    {
        select! {
            o = fut => {
                on_complete(o)
            },
            _ = self.cancel.cancelled() => {
                Err(reality::Error::PluginCallCancelled.into())
            }
        }
    }
}