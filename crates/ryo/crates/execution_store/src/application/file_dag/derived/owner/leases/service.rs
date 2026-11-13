//! The `leases` application operation.
use std::sync::Arc;

use super::super::service::*;
use crate::application::file_dag::accounting::ledger::service::Ledger;
use crate::application::file_dag::accounting::ledger::service::ResourceLease;
use crate::application::file_dag::accounting::ledger::types::{Backing, LeaseClass};
use ryo_execution_domain::domain::operating::errors::OperatingError;

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
