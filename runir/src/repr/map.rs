use super::*;

pub struct Map<'a, R: Repr> {
    pub(crate) table: &'a mut ReprTable<R>,
    pub(crate) handle: ReprHandle
}

impl<'a, R: Repr> Map<'a, R> {
    /// Maps an identifier for the current handle to a representation
    pub fn map<'c>(self, ident: impl Into<Identifier<'c>>, repr: R)
    where
        R: ReprInternals
    {
        if let Some(head) = self.table.tree.get_mut(self.handle) {
            *head = head.clone().map(ident, repr);
        }
    }
}