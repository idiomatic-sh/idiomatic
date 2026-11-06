//! Sole source-capability issuer; every field is private to this module.
use crate::application::file_dag::derived::ledger::{Backing, ResourceLease};
use crate::application::file_dag::derived::owner::DerivedOwner;
use crate::application::file_dag::derived::owner::acquisition::semantics::service::DerivedStorage;
use crate::application::file_dag::derived::population::types::PopulationId;
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, MembershipSpace},
    operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::DerivedKind;
use ryo_derived::domain::contract::types::DerivedView;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub(crate) struct AcquiredView {
    plan: Arc<DerivedContract>,
    storage: Arc<DerivedStorage>,
    lease: Arc<ResourceLease>,
    backings: Arc<[Backing]>,
    built: bool,
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
        built: bool,
    ) -> Self {
        Self {
            plan: Arc::clone(plan),
            storage,
            lease,
            backings,
            origin,
            built,
        }
    }
    pub(in crate::application::file_dag::derived::owner::acquisition) fn into_storage(
        self,
    ) -> Option<DerivedStorage> {
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
    live: Arc<crate::application::file_dag::relations::GenerationLease>,
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
        f.debug_struct("AcquisitionOrigin")
            .field("lifetime", &self.lifetime)
            .field("kind", &self.plan.kind)
            .finish_non_exhaustive()
    }
}

impl AcquisitionOrigin {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn validate(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
    ) -> Result<(), OperatingError> {
        if !Arc::ptr_eq(&self.owner, &owner.identity) || !self.live.is_live() {
            return Err(OperatingError::NotRead);
        }
        if !Arc::ptr_eq(&self.plan, plan) {
            return Err(OperatingError::WrongOperation);
        }
        Ok(())
    }
}

impl AdmittedViewInputs<'_> {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn validate(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
        inputs: &DerivedInputs<'_, '_>,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, plan)?;
        use crate::domain::operator::contract::views::BorrowedRows;
        let admitted_rows = match (&self.sources, inputs) {
            (AcquisitionSources::Raw, DerivedInputs::RawWords(_)) => return Ok(()),
            (
                AcquisitionSources::Rows(_),
                DerivedInputs::Membership {
                    space: MembershipSpace::Dense(_),
                    ..
                },
            ) => {
                return Err(OperatingError::NotRead);
            }
            (AcquisitionSources::Rows(rows), _) => rows,
            (
                AcquisitionSources::Dense { rows, graph },
                DerivedInputs::Membership {
                    space: MembershipSpace::Dense(values),
                    ..
                },
            ) if std::ptr::eq(graph.vertices(), *values) => rows,
            _ => return Err(OperatingError::NotRead),
        };
        let rows = match inputs {
            DerivedInputs::Rows(rows) | DerivedInputs::Text { rows, .. } => rows.to_vec(),
            DerivedInputs::Predicate { rows, .. }
            | DerivedInputs::Distinct { rows, .. }
            | DerivedInputs::Postings { rows, .. }
            | DerivedInputs::Partition { rows, .. }
            | DerivedInputs::Adjacency { rows, .. } => vec![*rows],
            DerivedInputs::Membership {
                rows,
                space: MembershipSpace::Native(tree),
            } => vec![*rows, BorrowedRows::Tree(*tree)],
            DerivedInputs::Membership { rows, .. } => vec![*rows],
            DerivedInputs::Dfs { roots, edges, .. } => vec![*roots, *edges],
            DerivedInputs::RawWords(_) => return Err(OperatingError::NotRead),
        };
        if rows.len() != admitted_rows.len()
            || !rows
                .into_iter()
                .zip(admitted_rows)
                .all(|(actual, admitted)| {
                    crate::application::file_dag::relations::same_admitted_rows(actual, *admitted)
                })
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag) fn admit_view<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        owner: &crate::application::file_dag::relations::RelationOwner,
        proofs: &[&crate::application::file_dag::relations::AdmittedInput<'a>],
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if !std::ptr::eq(self, &owner.derived) || proofs.is_empty() || proofs.len() > 2 {
            return Err(OperatingError::NotRead);
        }
        let populations = proofs
            .iter()
            .map(|proof| proof.identity(owner).map(|identity| identity.0))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AdmittedViewInputs {
            origin: AcquisitionOrigin {
                owner: Arc::clone(&self.identity),
                live: owner.generation_liveness(),
                plan: Arc::clone(plan),
                populations,
                lifetime: AcquisitionLifetime::Population,
            },
            sources: AcquisitionSources::Rows(proofs.iter().map(|proof| proof.rows()).collect()),
        })
    }
    pub(in crate::application::file_dag) fn admit_partition<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        owner: &crate::application::file_dag::relations::RelationOwner,
        proof: &crate::application::file_dag::relations::AdmittedInput<'a>,
        parent: &AcquiredView,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        parent.origin().validate(self, &parent.origin().plan)?;
        if parent.contract().kind != DerivedKind::PartitionOrder
            || plan.kind != DerivedKind::ClosureAdjacency
        {
            return Err(OperatingError::WrongOperation);
        }
        let mut admitted = self.admit_view(plan, owner, &[proof])?;
        if admitted.origin.populations != parent.origin().populations {
            return Err(OperatingError::NotRead);
        }
        admitted.origin.lifetime = AcquisitionLifetime::Invocation;
        Ok(admitted)
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner) fn admit_raw(
        &self,
        plan: &Arc<DerivedContract>,
        owner: &crate::application::file_dag::relations::RelationOwner,
        permit: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
    ) -> Result<AdmittedViewInputs<'static>, OperatingError> {
        permit.validate_derived_owner(owner)?;
        if !std::ptr::eq(self, &owner.derived) || plan.kind != DerivedKind::RawKeyOrder {
            return Err(OperatingError::WrongOperation);
        }
        Ok(AdmittedViewInputs {
            origin: AcquisitionOrigin {
                owner: Arc::clone(&self.identity),
                live: owner.generation_liveness(),
                plan: Arc::clone(plan),
                populations: Vec::new(),
                lifetime: AcquisitionLifetime::Invocation,
            },
            sources: AcquisitionSources::Raw,
        })
    }
}

impl AdmittedViewInputs<'_> {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn populations(
        &self,
    ) -> &[PopulationId] {
        &self.origin.populations
    }
}

impl DerivedOwner {
    fn graph_sources<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        let mut admitted = input.admission(plan)?;
        admitted.origin.populations = vec![input.population()?];
        if !input.canonical_backing() {
            admitted.origin.lifetime = AcquisitionLifetime::Invocation;
        }
        Ok(admitted)
    }
    pub(in crate::application::file_dag::derived::owner::acquisition) fn admit_graph<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if plan.kind != DerivedKind::Adjacency {
            return Err(OperatingError::WrongOperation);
        }
        self.graph_sources(plan, input)
    }
    pub(in crate::application::file_dag::derived::owner::acquisition) fn admit_domain<'a>(
        &self,
        plan: &Arc<DerivedContract>,
        input: &crate::application::file_dag::workspace::GraphInput<'a>,
        parent: &AcquiredView,
    ) -> Result<AdmittedViewInputs<'a>, OperatingError> {
        if plan.kind != DerivedKind::Membership {
            return Err(OperatingError::WrongOperation);
        }
        parent.origin().validate(self, &parent.origin().plan)?;
        let mut admitted = self.graph_sources(plan, input)?;
        if !Arc::ptr_eq(&parent.origin().live, &admitted.origin.live) {
            return Err(OperatingError::NotRead);
        }
        let graph = parent
            .storage()
            .payload()
            .share_adjacency()
            .ok_or(OperatingError::WrongOperation)?;
        let AcquisitionSources::Rows(rows) = admitted.sources else {
            return Err(OperatingError::WrongOperation);
        };
        admitted.sources = AcquisitionSources::Dense { rows, graph };
        let population = input.population()?;
        admitted.origin.populations = parent.origin().populations.clone();
        if !admitted.origin.populations.contains(&population) {
            admitted.origin.populations.push(population);
        }
        Ok(admitted)
    }
}

impl AdmittedViewInputs<'_> {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn lifetime(
        &self,
    ) -> AcquisitionLifetime {
        self.origin.lifetime
    }
    pub(in crate::application::file_dag::derived::owner::acquisition) fn validate_origin(
        &self,
        owner: &DerivedOwner,
        plan: &Arc<DerivedContract>,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, plan)
    }
}

impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn complete_view(
        &self,
        plan: &Arc<DerivedContract>,
        storage: Arc<DerivedStorage>,
        lease: Arc<ResourceLease>,
        backings: Arc<[Backing]>,
        admitted: &AdmittedViewInputs<'_>,
        built: bool,
    ) -> Result<AcquiredView, OperatingError> {
        admitted.origin.validate(self, plan)?;
        storage.view()?;
        Ok(AcquiredView::new(
            plan,
            storage,
            lease,
            backings,
            admitted.origin.clone(),
            built,
        ))
    }
}
impl AcquiredView {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn lease(
        &self,
    ) -> &Arc<ResourceLease> {
        &self.lease
    }
}

impl AcquiredView {
    pub(in crate::application::file_dag::derived::owner) fn validate_owner(
        &self,
        owner: &DerivedOwner,
    ) -> Result<(), OperatingError> {
        self.origin.validate(owner, &self.plan)
    }
}
