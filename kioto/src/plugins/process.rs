use std::{path::PathBuf, process::Output};

use reality::*;
use runplat_macros::kt_metadata;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::utils::with_cancel;

/// Plugin for starting a process
#[kt_metadata(loader)]
#[derive(Serialize, Deserialize, Resource)]
pub struct Process {
    /// Program to start a process for
    program: String,
    /// Program arguments
    args: Vec<String>,
    /// Bin dir to find the program from
    bin_dir: Option<PathBuf>,
    /// Output of the process
    #[serde(skip)]
    output: Option<Output>
}

impl Process {
    /// Takes the process output
    #[inline]
    pub fn take_output(&mut self) -> Option<Output> {
        self.output.take()
    }
}

impl Plugin for Process {
    fn call(bind: plugin::Bind<Self>) -> CallResult {
        if bind.receiver()?.output.is_some() {
            debug!("Process output has not been handled");
            return bind.skip();
        }

        bind.defer(|mut binding, ct| async move {
            let p = binding.receiver()?;
            let mut command = if let Some(bin_dir) = p.bin_dir.as_ref() {
                tokio::process::Command::new(bin_dir.join(&p.program))
            } else {
                tokio::process::Command::new(&p.program)
            };

            let checked = shlex::try_join(p.args.iter().map(|s| s.as_str()))
                .map_err(|e| binding.plugin_call_error(e.to_string()))?;
            if let Some(args) = shlex::split(&checked) {
                command.args(args);
            }

            let output = with_cancel(ct)
                .run(command.output())
                .await??;
            // eprintln!("Calling command {output:?}");
            let status = output.status;
            binding.update()?.output = Some(output);
            if status.success() {
                Ok(())
            } else {
                Err(binding.plugin_call_error(format!(
                    "process exited unsuccessfully, status code: {}",
                    status.code().unwrap_or(1)
                )))
            }
        })
    }

    fn version() -> Version {
        Version::new(0, 1, 0)
    }
}

impl Content for Process {
    fn state_uuid(&self) -> uuid::Uuid {
        BincodeContent::new(self).unwrap().state_uuid()
    }
}
