use crate::Resource;
use std::{
    any::TypeId,
    pin::Pin,
    sync::{Arc, RwLock},
};

use super::ObservationEvent;

/// Type-alias for a resource cell which stores a resource for an item
type ResourceCell = std::sync::Arc<std::sync::RwLock<PinnedResource<dyn Resource>>>;

/// Type-alias for a pinned resource
type PinnedResource<R> = Pin<Box<R>>;

/// Container for a single resource
pub struct Item {
    /// Contains a single resource
    cell: ResourceCell,
    /// Type-id of the stored resource
    type_id: TypeId,
    /// Optional, observation event
    observe: Option<ObservationEvent>,
}

impl Item {
    /// Borrows and casts a mutable reference for the inner resource
    ///
    /// Returns None if `T` does not match the stored resource
    pub fn borrow_mut<T: Resource>(&mut self) -> Option<&mut T> {
        if std::any::TypeId::of::<T>() == self.type_id {
            let mut resource = match self.cell.write() {
                Ok(guard) => guard,
                Err(err) => err.into_inner(),
            };

            let resource = resource.as_mut();
            unsafe {
                let inner = Pin::into_inner_unchecked(resource);
                let cast = cast_mut_ref(inner).cast::<T>();
                let cast = cast.as_mut();

                #[cfg(feature = "observe")]
                if cast.is_some() {
                    if let Some(obvs) = self.observe.take() {
                        let sync = &*obvs.sync;
                        let mut state = match sync.0.lock() {
                            Ok(g) => g,
                            Err(e) => e.into_inner(),
                        };
                        state.accessed = true;
                        sync.1.notify_one();
                        drop(state);
                    }
                }
                cast
            }
        } else {
            None
        }
    }

    /// Borrows and casts a reference for teh inner resource
    ///
    /// Returns None if `T` does not match the stored resource
    pub fn borrow<T: Resource>(&self) -> Option<&T> {
        if std::any::TypeId::of::<T>() == self.type_id {
            let mut resource = match self.cell.write() {
                Ok(guard) => guard,
                Err(err) => err.into_inner(),
            };

            let resource = resource.as_mut();
            unsafe {
                let inner = Pin::into_inner_unchecked(resource);
                let cast = cast_ref(inner).cast::<T>();
                cast.as_ref()
            }
        } else {
            None
        }
    }

    /// Observes access on the item
    #[cfg(feature = "observe")]
    pub fn observe(&mut self) -> ObservationEvent {
        let obvs = ObservationEvent::new();
        self.observe = Some(obvs.clone());
        obvs
    }

    /// Observes access on the item with a timeout
    #[cfg(feature = "observe")]
    pub fn observe_with_timeout(&mut self, timeout: Duration) -> ObservationEvent {
        let mut obvs = ObservationEvent::new();
        obvs.timeout(timeout);
        self.observe = Some(obvs.clone());
        obvs
    }
}

impl<R: Resource> From<R> for Item {
    fn from(value: R) -> Self {
        let type_id = value.type_id();
        Self {
            cell: Arc::new(RwLock::new(Box::pin(value))),
            type_id,
            observe: None,
        }
    }
}

impl Clone for Item {
    fn clone(&self) -> Self {
        Self {
            cell: self.cell.clone(),
            type_id: self.type_id.clone(),
            observe: self.observe.clone(),
        }
    }
}

/// Casts a mutable reference to a raw mutable pointer
fn cast_mut_ref<T: ?Sized>(r: &mut T) -> *mut T {
    r
}

/// Casts a reference to a raw const pointer
fn cast_ref<T: ?Sized>(r: &T) -> *const T {
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_item_borrow_resource() {
        let mut item = Item::from(String::from("HELLO WORLD"));
        if let Some(item) = item.borrow_mut::<String>() {
            item.extend(['t', 'e', 's', 't']);
        }
    
        let item = item.borrow::<String>().expect("should exist");
        assert_eq!("HELLO WORLDtest", item);
    }
}
