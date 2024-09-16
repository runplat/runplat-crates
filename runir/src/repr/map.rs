use super::*;

/// Struct for executing map actions on a repr table
pub struct Map<'a, R: Repr> {
    /// Table being edited
    pub(crate) repo: &'a mut Repo<R>,
    /// Handle being mapped with
    pub(crate) handle: ReprHandle,
}

impl<'a, R: Repr> Map<'a, R> {
    /// Maps an identifier for the current handle to a representation
    pub fn map<'c>(&mut self, ident: impl Into<Identifier<'c>>, repr: R) -> &mut Self {
        let handle = self.handle.clone();
        if let Some(head) = self.repo.get_mut(&handle) {
            *head = head.clone().map(handle, ident, repr);
        }
        self
    }
}
