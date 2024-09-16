use crate::Resource;

/// Struct containing key data for a resource
pub struct Key {
    /// Encoded data representing this key
    ///
    /// The UUID layout is used to store data on a key
    pub data: u128,
}
