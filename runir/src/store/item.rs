use crate::{repo::Journal, repr::Attributes, Resource};
use std::{
    any::TypeId,
    pin::Pin,
    sync::{Arc, RwLock}
};

use super::{observe::Observable, ObservationEvent};

/// Type-alias for a resource cell which stores a resource for an item
type ResourceCell = std::sync::Arc<std::sync::RwLock<PinnedResource<dyn Resource>>>;

/// Type-alias for a pinned resource
type PinnedResource<R> = Pin<Box<R>>;

/// Container for a single resource
#[derive(Clone)]
pub struct Item {
    /// Contains a single resource
    cell: ResourceCell,
    /// Type-id of the stored resource (TODO: Can store this in attributes)
    type_id: TypeId,
    /// Handle associated with this item and to the resource's attribute map
    journal: Journal,
    /// Commit id
    commit: u64,
    /// Optional, observation event
    observe: Option<ObservationEvent>,
}

impl Item {
    /// Creates a new item
    #[inline]
    pub fn new<R: Resource>(journal: Journal, commit: u64, resource: R) -> Self {
        let type_id = resource.type_id();
        Self {
            cell: Arc::new(RwLock::new(Box::pin(resource))),
            type_id,
            journal,
            commit,
            observe: None,
        }
    }

    /// Returns attributes for this item
    ///
    /// If an item exists, it means that it will have an associated "Attributes" store which points to various attributes owned by this resource.
    ///
    /// **Note**: An item currently, may only store a single type of each attribute.
    pub fn attributes(&self) -> Arc<Attributes> {
        self.journal
            .get(self.commit)
            .and_then(|h| h.cast())
            .expect("should always point to attributes")
    }

    /// Creates an observable
    pub fn observe(&self) -> Observable {
        let mut inner = self.clone();
        let event = ObservationEvent::new();
        inner.observe = Some(event.clone());
        Observable {
            inner,
            event,
        }
    }

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

    /// Returns true if this item matches the resource
    #[inline]
    pub fn is_type<T: Resource>(&self) -> bool {
        self.matches_type(std::any::TypeId::of::<T>())
    }

    /// Returns true if the a type id matches the current type id this item hosts
    #[inline]
    pub fn matches_type(&self, other: TypeId) -> bool {
        other == self.type_id
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
    use std::time::Duration;

    use crate::{repr::TyRepr, store::Store};

    #[test]
    fn test_item_borrow_resource() {
        let mut store = Store::new();
        let handle = store.put(String::from("HELLO WORLD")).commit();

        let mut item = store.item(handle.commit()).unwrap().clone();
        if let Some(item) = item.borrow_mut::<String>() {
            item.extend(['t', 'e', 's', 't']);
        }

        let item = item.borrow::<String>().expect("should exist");
        assert_eq!("HELLO WORLDtest", item);
    }

    #[test]
    fn test_item_attributes() {
        let mut store = Store::new();
        let handle = store
            .put(String::from("HELLO WORLD"))
            .attr(TyRepr::new::<u64>())
            .commit();

        let item = store.item(handle.commit()).unwrap();

        let test = item.attributes().get::<TyRepr>();
        assert!(test.is_some());
        assert_eq!(test.unwrap().as_ref(), &TyRepr::new::<u64>());
    }

    #[test]
    fn test_item_borrow_resource_multi_thread() {
        let mut store = Store::new();
        let handle = store.put(String::from("HELLO WORLD")).commit();

        let item = store.item(handle.commit()).unwrap().clone();
        let mut observable = item.observe();
        let mut observe = observable.event();
        let _ = std::thread::Builder::new().spawn(move || {
            observable.notify_start();
            let item = observable.borrow_mut::<String>().expect("should be able to borrow");
            item.extend(['t', 'e', 's', 't']);
            observable.notify_change_with_message("change happend");
            std::thread::sleep(Duration::from_secs(1));
            observable.notify_change_with_progress(100);
        });

        std::thread::sleep(Duration::from_millis(100));
        let mut last_state = observe.wait();
        while last_state.progress < 100 {
            last_state = observe.wait();
        }
        let item = item.borrow::<String>().expect("should exist");
        assert_eq!("HELLO WORLDtest", item);
        assert_eq!("change happend", last_state.message);
        assert_eq!(3, last_state.version);
    }

    #[test]
    fn test_item_borrow_resource_multi_thread_observe_timeout() {
        let mut store = Store::new();
        let handle = store.put(String::from("HELLO WORLD")).commit();
        let item = store.item(handle.commit()).unwrap().clone();
        let mut observe = item.observe();
        let mut event = observe.event();
        let _ = std::thread::Builder::new().spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            observe.borrow_mut::<String>().expect("should be able to borrow").extend(['b', 'a', 'd']);
        });
        event.timeout(Duration::from_millis(100));
        event.wait();
        let item = item.borrow::<String>().expect("should exist");
        assert_eq!("HELLO WORLD", item);
    }
}
