mod add;
mod checkout;
mod config;
mod journal;
mod map;
mod repo;
mod ty;

pub use add::Add;
pub use checkout::Checkout;
pub use config::Config;
pub use journal::Journal;
pub use map::Map;
pub use repo::Repo;
pub use ty::TyRepr;

use crate::Resource;
use std::{any::TypeId, borrow::Cow, collections::BTreeMap, fmt::Debug, pin::Pin, sync::Arc};

/// Type-alias for a dynamic head repr
type DynHead = Pin<Arc<dyn Repr>>;

/// Type-alias for the internal data structure used by the `Tree` type
type OrderedReprMap<R> = BTreeMap<u64, Head<R>>;

/// Enumeration of identifier variants
#[derive(Clone)]
pub enum Identifier<'a> {
    Unit,
    Str(Cow<'a, str>),
    Id(usize),
}

/// Representation is associated data that can be used to represent a resource in various contexts
///
/// For example, a resource's type information is it's representation within a rust application.
pub trait Repr: Resource {
    /// Returns a cast id for confirming that a DynHandle can cast into this repr
    fn cast_id(self: Arc<Self>) -> TypeId {
        self.as_ref().type_id()
    }

    /// Returns the internals implementation
    fn internals(&self) -> impl ReprInternals
    where
        Self: Sized,
    {
        TyRepr::new::<Self>()
    }
}

/// Representation internals required for managing repr maps and tables
pub trait ReprInternals: Sized + Repr {
    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_str(&self, identifier: &str) -> u64;

    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_id(&self, identifier: usize) -> u64;

    /// Returns the head value for this representation
    fn head(&self) -> Head<Self>;

    /// Returns a repr_handle for the current repr
    fn handle(&self) -> ReprHandle;
}

/// Struct containing the head representation value
pub struct Head<R> {
    pub inner: Pin<Arc<R>>,
    journal: Journal,
}

/// Enumeration of kinds of representations
pub enum Kind<R: Repr> {
    /// Internable representations are typically constant literal values that can be broadly represented,
    /// such as type information derived by the compiler. It is well suited for counting classification, but
    /// not well suited for representations that are unique per resource.
    Interned(Head<R>),
    /// Mappable representation where each resource can map to a unique repr value. This kind of representation requires an additional "next" value
    /// in addition to the "handle" value in order to succesfully store. It is well suited for naming resources or attaching other types of
    /// labeling data
    Mapped {
        /// Head value which can be used to derive identifier keys for mapped representations
        head: Head<R>,
        /// Thread-safe ordered map of identifiers mapped to Head values
        map: OrderedReprMap<R>,
    },
}

/// Handle containing lookup keys for storing representations
///
/// Can be used to access the representation directly later.
///
/// Also, the link value can be used to retrieve this handle from a journal,
/// if the handle was created from `checkout()`,
#[derive(Clone)]
pub struct ReprHandle {
    /// Handle value is the key of the head value
    handle: u64,
    /// Link value is the key of a mapped represntation
    link: u64,
    /// Pointer to the dynamic head representation
    head: DynHead,
}

impl<R> Head<R> {
    /// Creates a new repr head
    pub fn new(repr: R) -> Self {
        Head {
            inner: Arc::pin(repr),
            journal: Journal::new(),
        }
    }

    pub fn next(&self, repr: R) -> Self {
        Self {
            inner: Arc::pin(repr),
            journal: self.journal.clone(),
        }
    }
}

impl<R: Repr> Kind<R> {
    /// Returns the "interned" representation which is the head representation
    pub fn interned(&self) -> &Head<R> {
        match self {
            Kind::Interned(head) | Kind::Mapped { head, .. } => head,
        }
    }

    /// Maps a repr to an ident
    pub fn map<'a>(self, handle: ReprHandle, ident: impl Into<Identifier<'a>>, repr: R) -> Kind<R> {
        let ident = ident.into();
        let next = handle.link_to(ident);
        match self {
            Kind::Interned(head) => {
                let n = head.next(repr);
                Kind::Mapped {
                    head,
                    map: {
                        let mut map = BTreeMap::new();
                        map.insert(next.link(), n);
                        map
                    },
                }
            }
            Kind::Mapped { head, mut map } => {
                let n = head.next(repr);
                Kind::Mapped {
                    head,
                    map: {
                        map.insert(next.link(), n);
                        map
                    },
                }
            }
        }
    }

    /// Gets a mapped reprsentation from the current kind of repr
    ///
    /// If the current kind is `Internable` returns None
    pub fn get<'a>(
        &self,
        handle: ReprHandle,
        ident: impl Into<Identifier<'a>>,
    ) -> Option<(ReprHandle, &Head<R>)> {
        match self {
            Kind::Interned(h) => {
                let handle = handle.link_to(ident.into());
                Some((handle.checkout(h.clone()), h))
            },
            Kind::Mapped { map, .. } => {
                let handle = handle.link_to(ident.into());
                map.get(&handle.link())
                    .map(|v| (handle.checkout(v.clone()), v))
            }
        }
    }
}

impl ReprHandle {
    /// Returns the "handle" value representing this handle
    #[inline]
    pub fn handle(&self) -> u64 {
        self.handle
    }

    /// Returns the "link" value representing this handle
    #[inline]
    pub fn link(&self) -> u64 {
        self.link
    }

    /// Returns the "key" value representing this handle
    #[inline]
    pub fn key(&self) -> u128 {
        self.uuid().as_u128()
    }

    /// Returns the current handle as an UUID
    #[inline]
    pub fn uuid(&self) -> uuid::Uuid {
        uuid::Uuid::from_u64_pair(self.handle, self.link)
    }

    /// Casts the shared head to a representation
    ///
    /// If the target type is not the same as the current head, None is returned
    #[inline]
    pub fn cast<T: Repr>(&self) -> Option<Arc<T>> {
        let inner = unsafe { Pin::into_inner_unchecked(self.head.clone()) };
        let ident = inner.clone().cast_id();
        let addr = Arc::into_raw(inner);
        let inner = unsafe { Arc::<T>::from_raw(addr.cast::<T>()) };
        let matches = ident == inner.clone().cast_id();
        Some(inner).filter(|_| matches)
    }

    /// Returns a new repr handle w/ a `link` value set
    #[inline]
    pub(crate) fn link_to<'a>(&self, ident: impl Into<Identifier<'a>>) -> ReprHandle {
        let link = match ident.into() {
            Identifier::Str(ident) => self.head.internals().link_hash_str(&ident),
            Identifier::Id(ident) => self.head.internals().link_hash_id(ident),
            Identifier::Unit => 0,
        };

        ReprHandle {
            handle: self.handle,
            link: {
                if self.link > 0 && self.link != link {
                    self.link ^ link
                } else {
                    link
                }
            },
            head: self.head.clone(),
        }
    }

    /// Returns this handle with a different head pointer and link setting
    #[inline]
    pub(crate) fn checkout<R: Repr>(&self, head: Head<R>) -> ReprHandle {
        let _head = head.inner.clone();

        let mut handle = ReprHandle {
            handle: self.handle,
            link: 0,
            head: _head,
        };
        handle.link = head.journal.log(handle.clone());
        handle
    }
}

impl Resource for DynHead {}
impl Repr for DynHead {}

impl<R: Repr> Clone for Kind<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Interned(arg0) => Self::Interned(arg0.clone()),
            Self::Mapped { head, map } => Self::Mapped {
                head: head.clone(),
                map: map.clone(),
            },
        }
    }
}

impl<R> Clone for Head<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            journal: self.journal.clone(),
        }
    }
}

impl<R: Repr> From<Head<R>> for Kind<R> {
    fn from(value: Head<R>) -> Self {
        Self::Interned(value)
    }
}

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(Cow::from(value))
    }
}

impl<'a> From<usize> for Identifier<'a> {
    fn from(value: usize) -> Self {
        Identifier::Id(value)
    }
}

impl Debug for ReprHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReprHandle")
            .field("handle", &self.handle)
            .field("link", &self.link)
            .finish()
    }
}

impl PartialEq for ReprHandle {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.link == other.link
    }
}

impl PartialOrd for ReprHandle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.handle.partial_cmp(&other.handle) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.link.partial_cmp(&other.link)
    }
}
