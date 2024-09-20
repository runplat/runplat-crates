use std::path::PathBuf;

use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    env: String,
    root: PathBuf,
    src: PathBuf,
    #[serde(rename = "src-size")]
    src_size: u64,
    event: String,
    #[serde(rename = "crc-ms")]
    crc_ms: String
}

impl Metadata {
    pub fn split_for_env_loader(&self) -> (String, std::io::Result<PathBuf>) {
        (self.env.to_string(), Ok(self.root.clone()))
    }
}