use std::path::PathBuf;

use reality::*;
use runplat_macros::kt_metadata;
use serde::{Deserialize, Serialize};

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
}

impl Plugin for Process {
    fn call(bind: plugin::Bind<Self>) -> CallResult {
        bind.defer(|binding, ct| async move {
            let p = binding.plugin()?;
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

            with_cancel(ct)
                .run(command.status(), |s| {
                    let status = s?;
                    if status.success() {
                        Ok(())
                    } else {
                        Err(binding.plugin_call_error(format!(
                            "process exited unsuccessfully, status code: {}",
                            status.code().unwrap_or(1)
                        )))
                    }
                })
                .await?;

            Ok(())
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
