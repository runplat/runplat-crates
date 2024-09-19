use std::path::PathBuf;

use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct Metadata {
    env: String,
    src: PathBuf,
    #[serde(rename = "src-size")]
    src_size: u64,
    event: String,
    #[serde(rename = "crc-ms")]
    crc_ms: String
}
