//! Sole source-capability issuer; every field is private to this module.
use crate::application::file_dag::derived::ledger::service::ResourceLease;
use crate::application::file_dag::derived::ledger::types::Backing;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::service::DerivedStorage;
use crate::application::file_dag::derived::population::service::PopulationId;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::{
    derived_views::types::{DerivedInputs, MembershipSpace, contract::value::DerivedContract},
    operating::errors::OperatingError,
};
use std::sync::Arc;

#[cfg(test)]
use super::types::PartitionCapacity;
#[cfg(test)]
use super::types::StorageIdentity;
use crate::application::file_dag::derived::constants;
use crate::application::file_dag::derived::owner::acquisition::population::types::Construction;
/// A view the owner leased to one invocation: its contract, its storage and the lease that keeps it alive.
#[derive(Clone, Debug)]
pub struct AcquiredView {
    plan: Arc<DerivedContract>,
    storage: Arc<DerivedStorage>,
    lease: Arc<ResourceLease>,
    backings: Arc<[Backing]>,
    #[cfg(test)]
    construction: Construction,
    origin: AcquisitionOrigin,
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
            #[cfg(test)]
            construction,
        }
    }
    pub(crate) fn into_storage(self) -> Option<DerivedStorage> {
        let Self { storage, lease, .. } = self;
        drop(lease);
        match Arc::try_unwrap(storage) {
            Ok(storage) => Some(storage),
            Err(_) => None,
        }
    }
    #[cfg(test)]
    pub(crate) fn partition_order_capacity(&self) -> Result<PartitionCapacity, OperatingError> {
        if !matches!(self.view()?, DerivedView::PartitionOrder(_)) {
            return Err(OperatingError::WrongOperation);
        }
        self.storage
            .payload()
            .partition_order_capacity()
            .map(|capacity| PartitionCapacity(capacity.0))
            .ok_or(OperatingError::WrongOperation)
    }
    #[cfg(test)]
    pub(crate) fn construction(&self) -> Construction {
        self.construction
    }
    /// The contract the view was acquired under.
    pub fn contract(&self) -> &DerivedContract {
        &self.plan
    }
    #[cfg(test)]
    pub(crate) fn storage_identity(&self) -> StorageIdentity {
        StorageIdentity(Arc::as_ptr(&self.storage))
    }
    /// The lease invariant is structural: `lease` is an owned field, so the ledger entries of
    /// `backings` stay required-live for as long as this value exists (`ResourceLease::drop`
    /// is the only release). The test-only `assert_required` re-derives that from the ledger;
    /// it is not unconditional because it takes a `RefCell` borrow, scans the lease keys and
    /// does one hash lookup per backing on every read, which production reads must not pay.
    ///
    /// # Errors
    ///
    /// The derived port's refusal to view the stored payload.
    pub fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        #[cfg(test)]
        self.lease.assert_required(&self.backings);
        self.storage.view()
    }
    pub(crate) fn storage(&self) -> &Arc<DerivedStorage> {
        &self.storage
    }
    pub(crate) fn backings(&self) -> &Arc<[Backing]> {
        &self.backings
    }
}
impl AcquiredView {
    pub(crate) fn origin(&self) -> &AcquisitionOrigin {
        &self.origin
    }
}

/// Explicit lifetime of semantic work; finite invocation work has no reusable key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcquisitionLifetime {
    Population,
    Invocation,
}
#[derive(Clone)]
pub(crate) struct AcquisitionOrigin {
    owner: Arc<()>,
    live: Arc<crate::application::file_dag::relations::generation::service::GenerationLease>,
    plan: Arc<DerivedContract>,
    populations: Vec<PopulationId>,
    lifetime: AcquisitionLifetime,
}
/// Only acquisition admission binds actual immutable sources to their owner and descriptor.
pub struct AdmittedViewInputs<'a> {
    origin: AcquisitionOrigin,
    sources: AcquisitionSources<'a>,
}
enum AcquisitionSources<'a> {
    Rows(Vec<ryo_execution_domain::domain::table_model::types::RepresentationInput<'a>>),
    Dense {
        rows: Vec<ryo_execution_domain::domain::table_model::types::RepresentationInput<'a>>,
        graph: Arc<ryo_derived::domain::adjacency::types::compact_graph::value::CompactGraph>,
    },
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
    pub(crate) fn populations(&self) -> &[PopulationId] {
        &self.origin.populations
    }
}

impl AcquiredView {
    /// The lease that keeps the view alive.
    pub(crate) fn lease(&self) -> &Arc<ResourceLease> {
        &self.lease
    }
}

mod issuance;
