/// Trait representing a dynamic resource which must be stored and retrieved
pub trait Resource : std::any::Any + Send + Sync + 'static {}