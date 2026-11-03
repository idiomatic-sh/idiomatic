//! One allocation ledger serves publication, views and backend retention.
use super::super::ledger::{Backing, LeaseClass, Ledger, ResourceLease};
use super::*;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn required_lease(
        &self,
        backings: &[Backing],
    ) -> Result<Arc<ResourceLease>, OperatingError> {
        Ledger::acquire(&self.ledger, backings, LeaseClass::Required).map(Arc::new)
    }
    pub(in crate::application::file_dag) fn optional_lease(
        &self,
        backings: &[Backing],
    ) -> Result<Arc<ResourceLease>, OperatingError> {
        Ledger::acquire(&self.ledger, backings, LeaseClass::Optional).map(Arc::new)
    }
    pub(in crate::application::file_dag) fn node_optional_lease(
        &self,
        backings: &[Backing],
        limit: usize,
    ) -> Result<Option<Arc<ResourceLease>>, OperatingError> {
        let state = self.ledger.borrow();
        let additional = state.incremental_node_optional_bytes(backings)?;
        let allowed = state
            .snapshot()
            .node_optional_bytes
            .checked_add(additional)
            .is_some_and(|bytes| bytes <= limit);
        drop(state);
        if !allowed {
            return Ok(None);
        }
        Ledger::acquire(&self.ledger, backings, LeaseClass::NodeOptional)
            .map(|lease| Some(Arc::new(lease)))
    }
}
