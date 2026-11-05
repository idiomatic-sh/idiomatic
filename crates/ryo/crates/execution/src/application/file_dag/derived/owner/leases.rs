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
}
