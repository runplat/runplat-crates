mod config;
mod find;
mod map;
mod ty;

use config::Config;
use find::Find;
use map::Map;
use ty::TyRepr;

use crate::Resource;
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock, RwLock},
};

/// Type-alias for a shared head repr
pub type SharedHead = Arc<dyn Repr>;

/// Type-alias for the internal data structure used by the `Tree` type
type OrderedReprMap<R> = BTreeMap<u64, Arc<R>>;

/// Type-alias for a thread-safe, cloneable, shared key store
type SharedKeys = Arc<std::sync::RwLock<Tree<TyRepr>>>;

/// Type-alias for the container storing shared keys
type SharedKeysContainer = std::sync::OnceLock<SharedKeys>;

/// Enumeration of identifier variants
pub enum Identifier<'a> {
    Str(&'a str),
    Id(u64),
}

/// Representation is associated data that can be used to represent a resource in various contexts
///
/// For example, a resource's type information is it's representation within a rust application.
pub trait Repr: Send + Sync + 'static {
    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_str(self: Arc<Self>, identifier: &str) -> u64;

    /// Returns a "link" value of a representation instance given an identifier
    ///
    /// **Note**: Since this is a hash function, it must return the same value for the same identifier
    fn link_hash_u64(self: Arc<Self>, identifier: u64) -> u64;
}

/// Representation internals required for managing repr maps and tables
pub trait ReprInternals: Sized + Repr {
    /// Returns the head value for this representation
    fn head(&self) -> Head<Self>;

    /// Returns the "handle" value of a representation instance
    fn handle_of(repr: Kind<Self>) -> ReprHandle;
}

/// Struct for a global table storing handles that point to representation data
pub struct ReprTable<R: Repr> {
    /// Tree storing representations for this table
    tree: Tree<R>,
    /// Shared key lookup
    keys: SharedKeysContainer,
}

impl<R: Repr> ReprTable<R> {
    /// Creates a new repr table
    ///
    /// A Repr table stores representations and consists of the main tree which maps to each representation,
    /// and a shared "Keys" reference which is used to generate normalize lookup keys
    #[inline]
    pub const fn new() -> Self {
        Self {
            tree: Tree {
                inner: BTreeMap::new(),
            },
            keys: OnceLock::new(),
        }
    }

    /// Creates a new relative table storing a different representation but sharing the same key-base
    #[inline]
    pub fn create_relative<O: Repr>(&self) -> ReprTable<O> {
        let table = ReprTable::new();
        if let Err(_) = table.keys.set(self.shared_keys().clone()) {
            unreachable!("Should be a new table")
        }
        table
    }

    /// Finds a representation from this table for a resource
    #[inline]
    pub fn find<Res: Resource>(&self, resource: &Res) -> Find<'_, R>
    where
        R: ReprInternals,
    {
        let handle = self.type_handle(resource);

        Find {
            table: self,
            handle,
        }
    }

    /// Configures a representation from this table for a resource
    #[inline]
    pub fn config<Res: Resource>(&mut self, resource: &Res) -> Config<'_, R>
    where
        R: ReprInternals,
    {
        let handle = self.type_handle(resource);

        Config {
            table: self,
            handle,
        }
    }

    #[inline]
    fn type_handle<Res: Resource>(&self, resource: &Res) -> ReprHandle {
        let ty_repr = TyRepr::from(resource);

        TyRepr::handle_of(Kind::Internable(ty_repr.head()))
    }

    fn shared_keys(&self) -> &SharedKeys {
        self.keys.get_or_init(|| {
            Arc::new(RwLock::new(Tree {
                inner: BTreeMap::new(),
            }))
        })
    }
}

/// Enumeration of kinds of representations
pub enum Kind<R: Repr> {
    /// Internable representations are typically constant literal values that can be broadly represented,
    /// such as type information derived by the compiler. It is well suited for counting classification, but
    /// not well suited for representations that are unique per resource.
    Internable(Head<R>),
    /// Mappable representation where each resource can map to a unique repr value. This kind of representation requires an additional "next" value
    /// in addition to the "handle" value in order to succesfully store. It is well suited for naming resources or attaching other types of
    /// labeling data
    Mappable {
        /// Head value which can be used to derive identifier keys for mapped representations
        head: Head<R>,
        /// Thread-safe conccurrent ordered map of identified representations
        map: OrderedReprMap<R>,
    },
}

/// Struct containing the head representation value
pub struct Head<R>(pub Arc<R>);

/// Handle containing key data to a specific representation
pub struct ReprHandle {
    /// Handle value is the key of the head value
    handle: u64,
    /// Link value is the key of a mapped represntation
    link: u64,
    /// Pointer to the shared head representation
    shared: SharedHead,
}

/// Contains a tree of representations
#[derive(Clone)]
struct Tree<R: Repr> {
    inner: BTreeMap<u64, Kind<R>>,
}

impl<R: Repr + ReprInternals> Kind<R> {
    /// Returns the head representation
    pub fn head(&self) -> &Head<R> {
        match self {
            Kind::Internable(head) | Kind::Mappable { head, .. } => head,
        }
    }

    /// Maps a repr to an ident
    pub fn map<'a>(self, ident: impl Into<Identifier<'a>>, repr: R) -> Kind<R> {
        let ident = ident.into();
        let handle = R::handle_of(self.clone());
        let next = handle.link_to(ident);
        match self {
            Kind::Internable(head) => Kind::Mappable {
                head,
                map: {
                    let mut map = BTreeMap::new();
                    map.insert(next.link(), repr.into());
                    map
                },
            },
            Kind::Mappable { head, mut map } => Kind::Mappable {
                head,
                map: {
                    map.insert(next.link(), repr.into());
                    map
                },
            },
        }
    }

    /// Gets a mapped reprsentation from the current kind of repr
    ///
    /// If the current kind is `Internable` returns None
    pub fn get<'a>(&self, ident: impl Into<Identifier<'a>>) -> Option<(ReprHandle, &Arc<R>)> {
        match self {
            Kind::Internable(_) => None,
            Kind::Mappable { map, .. } => {
                let handle = R::handle_of(self.clone()).link_to(ident.into());
                map.get(&handle.link()).map(|v| (handle, v))
            }
        }
    }
}

impl ReprHandle {
    /// Returns a new repr handle w/ a `link` value set
    pub fn link_to(&self, ident: Identifier<'_>) -> ReprHandle {
        let shared = self.shared.clone();

        let link = match ident {
            Identifier::Str(ident) => shared.link_hash_str(ident),
            Identifier::Id(ident) => shared.link_hash_u64(ident),
        };

        ReprHandle {
            handle: self.handle,
            link,
            shared: self.shared.clone(),
        }
    }

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
}

impl<R: Repr> Tree<R> {
    // /// Inserts a "branch" into the map, returns the previous value if a previous entry existed
    // pub fn branch(&mut self, repr: R) -> Option<Kind<R>>
    // where
    //     R: ReprInternals,
    // {
    //     let head = repr.head();
    //     let branch = Kind::Internable(head);
    //     let handle = R::handle_of(branch.clone());
    //     self.inner.insert(handle.handle(), branch)
    // }

    /// Get a representation from a ReprHandle
    pub fn get(&self, handle: ReprHandle) -> Option<&Kind<R>> {
        self.inner.get(&handle.handle())
    }

    /// Get a representation from a ReprHandle
    pub fn get_mut(&mut self, handle: ReprHandle) -> Option<&mut Kind<R>> {
        self.inner.get_mut(&handle.handle())
    }
}

impl<R: Repr> Clone for Kind<R> {
    fn clone(&self) -> Self {
        match self {
            Self::Internable(arg0) => Self::Internable(arg0.clone()),
            Self::Mappable { head, map } => Self::Mappable {
                head: head.clone(),
                map: map.clone(),
            },
        }
    }
}

impl<R> Clone for Head<R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<R: Repr> From<Head<R>> for Kind<R> {
    fn from(value: Head<R>) -> Self {
        Self::Internable(value)
    }
}

impl<'a> From<&'a str> for Identifier<'a> {
    fn from(value: &'a str) -> Self {
        Self::Str(value)
    }
}

impl<'a> From<u64> for Identifier<'a> {
    fn from(value: u64) -> Self {
        Identifier::Id(value)
    }
}

impl<R: Repr> Default for ReprTable<R> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ty::TyRepr, Repr, ReprHandle, ReprInternals, ReprTable};
    use crate::Resource;
    use std::sync::Arc;

    #[derive(Default)]
    struct TestRepr;

    impl Resource for TestRepr {}

    impl Repr for TestRepr {
        fn link_hash_str(self: std::sync::Arc<Self>, identifier: &str) -> u64 {
            TyRepr::from(self.as_ref())
                .head()
                .0
                .link_hash_str(identifier)
        }

        fn link_hash_u64(self: std::sync::Arc<Self>, identifier: u64) -> u64 {
            TyRepr::from(self.as_ref())
                .head()
                .0
                .link_hash_u64(identifier)
        }
    }

    impl ReprInternals for TestRepr {
        fn head(&self) -> super::Head<Self> {
            super::Head(Arc::new(TestRepr))
        }

        fn handle_of(repr: super::Kind<Self>) -> super::ReprHandle {
            match repr {
                super::Kind::Internable(head) => ReprHandle {
                    handle: TyRepr::handle_of(super::Kind::Internable(
                        TyRepr::from(head.0.as_ref()).head(),
                    ))
                    .handle(),
                    link: 0,
                    shared: head.0.clone(),
                },
                super::Kind::Mappable { head, .. } => ReprHandle {
                    handle: TyRepr::handle_of(super::Kind::Internable(
                        TyRepr::from(head.0.as_ref()).head(),
                    ))
                    .handle(),
                    link: 0,
                    shared: head.0.clone(),
                },
            }
        }
    }

    struct TestResource;

    impl Resource for TestResource {}

    #[test]
    fn test_repr_table() {
        let mut table = ReprTable::<TestRepr>::new();
        table
            .config(&TestResource)
            .default_mapped()
            .map("test", TestRepr);
        let _ = table
            .find(&TestResource)
            .ident("test")
            .expect("should exist");

        let test_string = String::from("hello world");
        table
            .config(&test_string)
            .default_mapped()
            .map("test2", TestRepr);

        let _ = table
            .find(&test_string)
            .ident("test2")
            .expect("should exist");
    }
}
