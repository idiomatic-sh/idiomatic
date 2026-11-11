//! One allocation ledger serves publication, views and backend retention.
use super::super::ledger::types::{Backing, LeaseClass};
use super::*;
use crate::application::file_dag::derived::ledger::service::Ledger;
use crate::application::file_dag::derived::ledger::service::ResourceLease;
impl DerivedOwner {
    pub(crate) fn required_lease(
        &self,
        backings: &[Backing],
    ) -> Result<Arc<ResourceLease>, OperatingError> {
        Ledger::acquire(&self.ledger, backings, LeaseClass::Required).map(Arc::new)
    }
    pub(crate) fn optional_lease(
        &self,
        backings: &[Backing],
    ) -> Result<Arc<ResourceLease>, OperatingError> {
        Ledger::acquire(&self.ledger, backings, LeaseClass::Optional).map(Arc::new)
    }
}
