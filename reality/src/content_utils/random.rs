use uuid::Uuid;

/// Content implementation that generates a random state_uuid each time
/// the resource is put into state
pub struct RandomContent;

impl<S> From<&S> for RandomContent {
    fn from(_: &S) -> Self {
        RandomContent
    }
}

impl runir::Content for RandomContent {
    fn state_uuid(&self) -> uuid::Uuid {
        Uuid::new_v4()
    }
}
