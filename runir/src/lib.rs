//! # Runtime Intermediate Representation
//! 
//! This library is for building an runtime intermediate representations of resources and their associated representations consumed during runtime.
//! 
//! The three main modules are `store`, `repo`, and `repr`.
//! 
//! The entrypoint to use this library is the "Store" type. The store centralizes resource state in a single location returning "Handles" that can be used to fetch
//! metadata for stored resources.

pub mod repr;
pub use repr::repo;
pub mod store;

/// Trait representing a dynamic resource which can be stored and retrieved
pub trait Resource: std::any::Any + Send + Sync + 'static {}

impl Resource for String {}
impl Resource for bool {}
impl Resource for u128 {}
impl Resource for u64 {}
impl Resource for u32 {}
impl Resource for usize {}
impl Resource for f64 {}
impl Resource for f32 {}
