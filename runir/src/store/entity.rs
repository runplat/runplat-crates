use std::sync::Arc;

#[derive(Default)]
pub struct Entity {
    inner: Arc<()>
}

impl Entity {
    /// Returns true if the entity can be destroyed
    pub fn can_destroy(&self) -> bool {
        Arc::strong_count(&self.inner) == 1
    }
}

#[test]
fn test_entity_can_destroy() {
    let e = Entity::default();

    assert!(e.can_destroy());
}

#[test]
fn test_entity_inner_unwrap_assertion() {
    let e = Entity::default();

    assert!(Arc::try_unwrap(e.inner).is_ok());
}