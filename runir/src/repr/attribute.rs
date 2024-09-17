use super::*;
use repo::{Handle, Journal};
use std::{collections::BTreeMap, sync::Arc};

/// Maps attribute typtes to their commit id in the journal
#[derive(Clone, Serialize)]
pub struct Attributes {
    /// Map of associated attributes commits
    attrs: BTreeMap<u64, u64>,
    /// Journal for accessing attributes
    #[serde(skip)]
    journal: Journal,
}

impl Attributes {
    /// Creates a new journal
    #[inline]
    pub const fn new(journal: Journal) -> Self {
        Self {
            attrs: BTreeMap::new(),
            journal,
        }
    }

    /// Inserts a new handle into attributes
    #[inline]
    pub fn insert<Attribute: Repr>(&mut self, handle: &Handle) {
        self.attrs
            .insert(Self::get_ty_bits::<Attribute>(), handle.commit());
    }

    /// Gets an attribute
    #[inline]
    pub fn get<Attribute: Repr>(&self) -> Option<Arc<Attribute>> {
        self.attrs
            .get(&Self::get_ty_bits::<Attribute>())
            .and_then(|a| self.journal.get(*a).and_then(|r| r.cast::<Attribute>()))
    }

    /// Returns the hi bits for an attribute
    fn get_ty_bits<Attribute: Repr>() -> u64 {
        let (hi, _) = TyRepr::new::<Attribute>()
            .hash_uuid::<Attribute>()
            .as_u64_pair();
        hi
    }
}

impl Resource for Attributes {}
impl Repr for Attributes {}
