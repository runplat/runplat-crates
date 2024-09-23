use std::path::PathBuf;

use serde::Serialize;

use super::Name;

/// Full address to a plugin which includes both the name and the commit the plugin is stored at
#[derive(Clone, Debug, Serialize)]
pub struct Address {
    /// Plugin name this address points to
    pub(crate) name: Name,
    /// Commit id the plugin registered as
    pub(crate) commit: u64,
}

impl Address {
    /// Commit id of this address
    #[inline]
    pub fn commit(&self) -> u64 {
        self.commit
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path: PathBuf = self.into();
        let path = path.to_string_lossy();
        write!(f, "{path}")
    }
}

impl From<&Address> for PathBuf {
    fn from(value: &Address) -> Self {
        value
            .name
            .path()
            .join(hex::encode(value.commit.to_be_bytes()))
    }
}
