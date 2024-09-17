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

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self
            .name
            .path()
            .join(hex::encode(self.commit.to_be_bytes()));
        let path = path.to_string_lossy();
        write!(f, "{path}")
    }
}
