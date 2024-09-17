use super::Name;

/// Full address to a plugin which includes both the name and the commit the plugin is stored at
#[derive(Debug)]
pub struct Address {
    pub(crate) name: Name,
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
