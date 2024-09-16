use super::*;

/// Type-alias for a dynamic head repr
type DynHead = Pin<Arc<dyn Repr>>;

/// Handle containing lookup keys for storing representations
///
/// Can be used to access the representation directly later.
///
/// Also, the link value can be used to retrieve this handle from a journal,
/// if the handle was created from `checkout()`,
#[derive(Clone)]
pub struct Handle {
    /// Commit value of the head this handle was created from
    commit: u64,
    /// Type-id for valid casting of this handle to the underlying representation
    cast: TypeId,
    /// Pointer to the dynamic head representation
    repr: DynHead,
}

impl Handle {
    /// Creates a new handle
    pub fn new<R: Repr>(commit: u64, repr: DynHead) -> Self {
        Self {
            commit,
            cast: TypeId::of::<R>(),
            repr,
        }
    }

    /// Returns the "commit" value this handle represents
    #[inline]
    pub fn commit(&self) -> u64 {
        self.commit
    }

    /// Returns a reference to the dynamic-repr
    #[inline]
    pub fn head(&self) -> &DynHead {
        &self.repr
    }

    /// Casts the shared head to a representation
    ///
    /// If the target type is not the same as the current head, None is returned
    #[inline]
    pub fn cast<T: Repr>(&self) -> Option<Arc<T>> {
        if TypeId::of::<T>() != self.cast {
            return None;
        }
        unsafe {
            let inner = Pin::into_inner_unchecked(self.repr.clone());
            let addr = Arc::into_raw(inner);
            let inner = Arc::<T>::from_raw(addr.cast::<T>());
            Some(inner)
        }
    }
}

impl Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReprHandle")
            .field("handle", &self.commit)
            .finish()
    }
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.commit == other.commit
    }
}

impl PartialOrd for Handle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.commit.partial_cmp(&other.commit)
    }
}

impl Resource for DynHead {}
impl Repr for DynHead {}
