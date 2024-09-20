use uuid::Uuid;

/// Content implementation that returns a nil uuid each time the resource is put into state
pub struct NilContent;

impl<S> From<&S> for NilContent {
    fn from(_: &S) -> Self {
        NilContent
    }
}

impl runir::Content for NilContent {
    fn state_uuid(&self) -> uuid::Uuid {
        Uuid::nil()
    }
}