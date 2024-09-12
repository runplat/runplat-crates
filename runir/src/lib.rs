//! Runtime Intermediate Representation
//! 
//! Key --> Resource
//! 
//! Key: [Attribute]
//! 
//! Attribute --> Repr

mod repr;
mod key;
mod attribute;
mod resource;

pub use key::Key;
pub use resource::Resource;


/// Trait describing runtime storage of resources
pub trait Storage {
    /// Associated type containing a resource
    type Cell<T> where T: Resource;

    /// Container for borrowing a resource from the storage target
    type BorrowResource<'a, T: Resource>: std::ops::Deref<Target = T> + Send + Sync
    where
        Self: 'a;

    /// Container for mutably borrowing a resource from the storage target
    type BorrowMutResource<'a, T: Resource>: std::ops::Deref<Target = T>
        + std::ops::DerefMut<Target = T>
        + Send
        + Sync
    where
        Self: 'a;


    /// Put a resource into storage
    /// 
    /// If the resource could be put into storage, a key can be returned for further configuration
    fn put_resource<'a: 'b, 'b, T: Resource>(
        &'a mut self,
        resource: T
    ) -> Option<&'b mut Key>;
}