//! One file generation owns representation descriptors and lazy admission proofs.
use super::derived::population::types::PopulationId;

use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_model::collections::representations::types::{
    PhysicalShape, RepresentationKind,
};
use ryo_execution_domain::domain::table_model::{
    service::{kind, materialize, registered},
    types::{PublishedRelation, Rows},
};
use ryo_execution_domain::domain::{executable_program::types::RegionId, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::{
    cell::{Cell, RefCell},
    sync::{Arc, OnceLock},
};
use rustc_hash::FxHashMap;
mod accounting;
pub mod types;
pub(in crate::application::file_dag) mod inspection;
pub(in crate::application::file_dag) mod producer;
pub(in crate::application::file_dag) mod source;
pub mod generation;
pub mod publication;
use admission::service::BorrowedInput;
use generation::service::GenerationLease;
use handle::service::PublishedInput;
use number::service::AdmittedNumber;
use packed::service::PackedPublication;
use publication::types::{
    ProducerKey, ProducerPopulation, RelationBody, RelationResult, SourceDescriptor,
};
use types::{CheckCount, PinnedViewCount, ReuseEligibility, Sameness};
use ryo_values::domain::rows::types::Table;
pub mod admission;
mod external;
pub mod handle;
mod identity;

pub mod number;
pub mod packed;

struct SourceViews {
    population: SourceDescriptor,
    canonical_population: PopulationId,
    views: FxHashMap<SourceDescriptor, PublishedInput>,
}

/// The relation owner of one file generation: it publishes relations, admits borrowed inputs and issues the generation they are bound to.
pub struct RelationOwner {
    #[cfg(test)]
    pub(super) identity_checks: Cell<CheckCount>,
    derived: super::derived::owner::DerivedOwner,
    next_population: Cell<PopulationId>,
    producer_populations: RefCell<FxHashMap<ProducerKey, ProducerPopulation>>,
    generation: RefCell<Arc<()>>,
    generation_live: RefCell<Arc<GenerationLease>>,
    stable: ReuseEligibility,
    sources: RefCell<FxHashMap<Relation, SourceViews>>,
    max_views: PinnedViewCount,
    views: Cell<PinnedViewCount>,
    checks: Cell<CheckCount>,
    reuses: Cell<CheckCount>,
    /// The route profile of the scope that opened this owner.
    routes: super::route_counts::types::RouteProfile,
}
impl RelationOwner {
    /// The derived-view owner of this generation.
    pub fn derived(&self) -> &super::derived::owner::DerivedOwner {
        &self.derived
    }
    /// The route profile the opening scope handed down, for this owner's work to record into.
    pub fn routes(
        &self,
    ) -> &super::route_counts::types::RouteProfile {
        &self.routes
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn derived_mut(
        &mut self,
    ) -> &mut super::derived::owner::DerivedOwner {
        &mut self.derived
    }
    /// The generation of the resident population, for a proof to keep and present back.
    pub fn generation_guard(&self) -> Arc<()> {
        Arc::clone(&self.generation.borrow())
    }
    /// The revocation lease of the resident population.
    pub fn generation_liveness(&self) -> Arc<GenerationLease> {
        Arc::clone(&self.generation_live.borrow())
    }
    /// Whether `generation` is the resident population's generation.
    pub fn matches_generation(
        &self,
        generation: &Arc<()>,
    ) -> Sameness {
        if Arc::ptr_eq(generation, &self.generation.borrow()) {
            Sameness::Same
        } else {
            Sameness::Different
        }
    }

    pub(super) fn for_scope(
        permit: super::scope::types::ScopePermit,
        views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
        routes: super::route_counts::types::RouteProfile,
    ) -> Self {
        Self::with_limits(ReuseEligibility::from(permit), views, semantic, routes)
    }
    /// An owner over `semantic` with the default pin limit and no route recording, for the proofs of the store and the engine.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn new(
        stable: ReuseEligibility,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, Default::default())
    }
    /// An owner whose work records into `routes`.
    #[cfg(test)]
    pub(in crate::application) fn recording(
        stable: ReuseEligibility,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
        routes: super::route_counts::types::RouteProfile,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, routes)
    }
    #[cfg(test)]
    pub(in crate::application) fn for_request(
        stable: ReuseEligibility,
        views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, views, semantic, Default::default())
    }
    fn with_limits(
        stable: ReuseEligibility,
        max_views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
        routes: super::route_counts::types::RouteProfile,
    ) -> Self {
        let max_derived = super::derived::owner::types::EntryLimit(
            max_views
                .0
                .saturating_mul(ryo_derived::domain::request::types::DerivedKind::ALL.len()),
        );
        Self {
            #[cfg(test)]
            identity_checks: Cell::new(CheckCount(0)),
            derived: super::derived::owner::DerivedOwner::new(max_derived, semantic, routes.clone()),
            next_population: Cell::new(PopulationId(0)),
            producer_populations: RefCell::new(FxHashMap::default()),
            generation: RefCell::new(Arc::new(())),
            generation_live: RefCell::new(Arc::new(GenerationLease::new())),
            stable,
            sources: RefCell::new(FxHashMap::default()),
            max_views,
            views: Cell::new(PinnedViewCount(0)),
            checks: Cell::new(CheckCount(0)),
            reuses: Cell::new(CheckCount(0)),
            routes,
        }
    }
    /// Whether this owner's publications may be reused across invocations.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn stable(&self) -> ReuseEligibility {
        self.stable
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub fn release_population(&self) {
        self.derived.clear();
        self.producer_populations.borrow_mut().clear();
        self.sources.borrow_mut().clear();
        self.views.set(PinnedViewCount(0));
        self.generation_live.borrow().invalidate();
        *self.generation_live.borrow_mut() = Arc::new(GenerationLease::new());
        *self.generation.borrow_mut() = Arc::new(());
    }
}

#[cfg(test)]
mod owner_test;

pub(in crate::application::file_dag) fn same_admitted_rows(
    left: RepresentationInput<'_>,
    right: RepresentationInput<'_>,
) -> Sameness {
    identity::same(left, right)
}
