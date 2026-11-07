//! Sole source-capability issuer; every field is private to this module.
use crate::application::file_dag::derived::ledger::types::{Backing, ResourceLease};
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::domain::{
    derived_views::types::{contract::value::DerivedContract, DerivedInputs, MembershipSpace},
    operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_derived::domain::contract::types::DerivedView;
use std::sync::Arc;

#[cfg(test)]
use super::types::PartitionCapacity;
#[cfg(test)]
use super::types::StorageIdentity;
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
#[derive(Clone, Debug)]
pub(crate) struct AcquiredView {
    plan: Arc<DerivedContract>,
    storage: Arc<DerivedStorage>,
    lease: Arc<ResourceLease>,
    backings: Arc<[Backing]>,
    construction: Construction,
    origin: AcquisitionOrigin,
}

/// Immutable payload dependency, independently leased by each publication role.
#[derive(Clone, Debug)]
pub(in crate::application::file_dag) struct ResourceDependency {
    _storage: Arc<DerivedStorage>,
    backings: Arc<[Backing]>,
}

impl AcquiredView {
    fn new(
        plan: &Arc<DerivedContract>,
        storage: Arc<DerivedStorage>,
        lease: Arc<ResourceLease>,
        backings: Arc<[Backing]>,
        origin: AcquisitionOrigin,
        construction: Construction,
    ) -> Self {
        Self {
            plan: Arc::clone(plan),
            storage,
            lease,
            backings,
            origin,
            construction,
        }
    }
    pub(in crate::application::file_dag::derived::owner::acquisition) fn into_storage(
        self,
    ) -> Option<DerivedStorage> {
        let Self { storage, lease, .. } = self;
        drop(lease);
        match Arc::try_unwrap(storage) {
            Ok(storage) => Some(storage),
            Err(_) => None,
        }
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
    ) -> Result<PartitionCapacity, OperatingError> {
        if !matches!(self.view()?, DerivedView::PartitionOrder(_)) {
            return Err(OperatingError::WrongOperation);
        }
        self.storage
            .payload()
            .partition_order_capacity()
            .map(PartitionCapacity)
            .ok_or(OperatingError::WrongOperation)
    }
    pub(in crate::application::file_dag) fn construction(&self) -> Construction {
        self.construction
    }
    pub(in crate::application::file_dag) fn contract(&self) -> &DerivedContract {
        &self.plan
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn storage_identity(&self) -> StorageIdentity {
        StorageIdentity(Arc::as_ptr(&self.storage))
    }
    /// The lease invariant is structural: `lease` is an owned field, so the ledger entries of
    /// `backings` stay required-live for as long as this value exists (`ResourceLease::drop`
    /// is the only release). The test-only `assert_required` re-derives that from the ledger;
    /// it is not unconditional because it takes a `RefCell` borrow, scans the lease keys and
    /// does one hash lookup per backing on every read, which production reads must not pay.
    pub(in crate::application::file_dag) fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        #[cfg(test)]
        self.lease.assert_required(&self.backings);
        self.storage.view()
    }
    pub(in crate::application::file_dag::derived::owner) fn storage(&self) -> &Arc<DerivedStorage> {
        &self.storage
    }
    pub(in crate::application::file_dag::derived::owner) fn backings(&self) -> &Arc<[Backing]> {
        &self.backings
    }
}
impl ResourceDependency {
    pub(in crate::application::file_dag) fn backings(&self) -> &[Backing] {
        &self.backings
    }
}
impl AcquiredView {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn origin(
        &self,
    ) -> &AcquisitionOrigin {
        &self.origin
    }
}

/// Explicit lifetime of semantic work; finite invocation work has no reusable key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) enum AcquisitionLifetime {
    Population,
    Invocation,
}
#[derive(Clone)]
pub(in crate::application::file_dag::derived::owner::acquisition) struct AcquisitionOrigin {
    owner: Arc<()>,
    live: Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    plan: Arc<DerivedContract>,
    populations: Vec<crate::application::file_dag::derived::population::types::PopulationId>,
    lifetime: AcquisitionLifetime,
}
/// Only acquisition admission binds actual immutable sources to their owner and descriptor.
pub(in crate::application::file_dag) struct AdmittedViewInputs<'a> {
    origin: AcquisitionOrigin,
    sources: AcquisitionSources<'a>,
}
enum AcquisitionSources<'a> {
    Rows(Vec<crate::domain::operator::contract::views::BorrowedRows<'a>>),
    Dense {
        rows: Vec<crate::domain::operator::contract::views::BorrowedRows<'a>>,
        graph: Arc<ryo_derived::domain::contract::types::CompactGraph>,
    },
    Raw,
}

impl std::fmt::Debug for AcquisitionOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(constants::ORIGIN_TYPE)
            .field(constants::ORIGIN_LIFETIME, &self.lifetime)
            .field(constants::ORIGIN_KIND, &self.plan.kind)
            .finish_non_exhaustive()
    }
}

impl AdmittedViewInputs<'_> {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn populations(
        &self,
    ) -> &[PopulationId] {
        &self.origin.populations
    }
}

impl AcquiredView {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn lease(
        &self,
    ) -> &Arc<ResourceLease> {
        &self.lease
    }
}

mod issuance;
