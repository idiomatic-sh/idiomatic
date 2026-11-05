//! Opaque request-bound resource lease; primitives receive only borrowed trait views.
use super::ledger::{Backing, ResourceLease};
use super::storage::DerivedStorage;
use crate::domain::{derived_views::types::DerivedContract, operating::errors::OperatingError};
use ryo_derived::domain::contract::types::DerivedView;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub(crate) struct AcquiredView {
    pub(super) plan: Arc<DerivedContract>,
    pub(super) storage: Arc<DerivedStorage>,
    pub(super) lease: Arc<ResourceLease>,
    pub(super) backings: Arc<[Backing]>,
    pub(super) built: bool,
}
impl AcquiredView {
    pub(in crate::application::file_dag::derived) fn new(
        plan: &Arc<DerivedContract>,
        storage: Arc<DerivedStorage>,
        lease: Arc<ResourceLease>,
        backings: Arc<[Backing]>,
        built: bool,
    ) -> Self {
        Self {
            plan: Arc::clone(plan),
            storage,
            lease,
            backings,
            built,
        }
    }
    pub(in crate::application::file_dag::derived) fn into_storage(self) -> Option<DerivedStorage> {
        let Self { storage, lease, .. } = self;
        drop(lease);
        Arc::try_unwrap(storage).ok()
    }
    pub(in crate::application::file_dag) fn dependency(&self) -> ResourceDependency {
        ResourceDependency {
            _storage: Arc::clone(&self.storage),
            backings: Arc::clone(&self.backings),
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn partition_order_capacity(
        &self,
    ) -> Result<usize, OperatingError> {
        if !matches!(self.view()?, DerivedView::PartitionOrder(_)) {
            return Err(OperatingError::WrongOperation);
        }
        self.storage
            .payload()
            .partition_order_capacity()
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag) fn was_built(&self) -> bool {
        self.built
    }
    pub(in crate::application::file_dag) fn contract(&self) -> &DerivedContract {
        &self.plan
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn same_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.storage, &other.storage)
    }
    pub(in crate::application::file_dag) fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        self.storage.view()
    }
}

/// Immutable payload dependency, independently leased by each publication role.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag) struct ResourceDependency {
    _storage: Arc<DerivedStorage>,
    pub(in crate::application::file_dag) backings: Arc<[Backing]>,
}
