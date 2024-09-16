use std::{collections::BTreeMap, fmt::Debug, hash::Hash, sync::Arc};

use repo::{Handle, Journal};

use super::*;

#[derive(Clone)]
pub struct Attributes {
    attrs: BTreeMap<u64, u64>,
    journal: Journal,
}

impl Hash for Attributes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.attrs.hash(state);
    }
}

impl Attributes {
    /// Creates a new journal
    pub const fn new(journal: Journal) -> Self {
        Self {
            attrs: BTreeMap::new(),
            journal,
        }
    }

    /// Inserts a new handle into attributes
    #[inline]
    pub fn insert<R: Repr>(&mut self, handle: &Handle) {
        self.attrs.insert(Self::get_ty_bits::<R>(), handle.commit());
    }

    /// Gets an attribute
    #[inline]
    pub fn get<R: Repr>(&self) -> Option<Arc<R>> {
        self.attrs
            .get(&Self::get_ty_bits::<R>())
            .and_then(|a| self.journal.get(*a).and_then(|r| r.cast::<R>()))
    }

    fn get_ty_bits<R: Repr>() -> u64 {
        let (hi, _) = TyRepr::new::<R>().hash_uuid::<R>().as_u64_pair();
        hi
    }
}

impl Debug for Attributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attributes")
            .field("attrs", &self.attrs)
            .finish()
    }
}

impl Resource for Attributes {}
impl Repr for Attributes {}
