/// Trait representing a dynamic resource which can be stored and retrieved
pub trait Resource : std::any::Any + Send + Sync + 'static {}