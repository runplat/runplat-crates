use std::{collections::BTreeMap, fmt::Debug, sync::Arc};

use crate::{Journal, Repr, ReprInternals, Resource, TyRepr};

#[derive(Clone)]
pub struct Attributes {
    pub(crate) attrs: BTreeMap<u64, u64>,
    journal: Journal,
}

impl Attributes {
    pub fn new(journal: Journal) -> Self {
        Self {
            attrs: BTreeMap::new(),
            journal,
        }
    }
    /// Gets an attribute
    #[inline]
    pub fn get<R: Repr>(&self) -> Option<Arc<R>> {
        let attr = TyRepr::new::<R>();
        self.attrs
            .get(&attr.handle().handle())
            .and_then(|l| self.journal.get(*l))
            .and_then(|l| l.cast::<R>())
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
