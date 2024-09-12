use crate::{attribute::Attribute, Resource};

/// Struct containing key data for a resource
pub struct Key {
    /// Encoded data representing this key
    /// 
    /// The UUID layout is used to store data on a key
    pub data: u128
}

struct KeyTable {}

impl Key {
    /// Stores an attribute with this key
    pub fn with(&mut self, attr: impl Into<Attribute>) -> &mut Self {
        self
    }
}

impl<R> From<&R> for Key 
where 
    R: Resource
{
    fn from(value: &R) -> Self {
        let id = value.type_id();
        
        todo!()
    }
}