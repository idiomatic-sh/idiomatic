//! One file generation owns representation descriptors and lazy admission proofs.
use super::derived::population::types::PopulationId;

use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use crate::domain::table_model::collections::representations::types::{
    PhysicalShape, RepresentationKind,
};
use crate::domain::table_model::{
    service::{kind, materialize, registered},
    types::{PublishedRelation, Rows, SelectedRows, StoredSelection},
};
use crate::domain::{executable_program::types::RegionId, operating::errors::OperatingError};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    sync::{Arc, OnceLock},
};
mod accounting;
pub(crate) mod types;
pub(in crate::application::file_dag) mod inspection;
pub(in crate::application::file_dag) mod producer;
pub(in crate::application::file_dag) mod source;
pub(in crate::application::file_dag) mod generation;
pub(crate) mod publication;
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
pub(in crate::application::file_dag) mod admission;
mod external;
pub(in crate::application::file_dag) mod handle;
mod identity;
pub(super) mod positioned;
mod selected_access;
pub(super) mod selection;

pub(in crate::application::file_dag) mod number;
pub(in crate::application::file_dag) mod packed;

struct SourceViews {
    population: SourceDescriptor,
    canonical_population: PopulationId,
    views: HashMap<SourceDescriptor, PublishedInput>,
}

pub(crate) struct RelationOwner {
    #[cfg(test)]
    pub(super) identity_checks: Cell<CheckCount>,
    derived: super::derived::owner::DerivedOwner,
    next_population: Cell<PopulationId>,
    producer_populations: RefCell<HashMap<ProducerKey, ProducerPopulation>>,
    generation: RefCell<Arc<()>>,
    generation_live: RefCell<Arc<GenerationLease>>,
    stable: ReuseEligibility,
    sources: RefCell<HashMap<Relation, SourceViews>>,
    max_views: PinnedViewCount,
    views: Cell<PinnedViewCount>,
    pub(in crate::application::file_dag) checks: Cell<CheckCount>,
    pub(in crate::application::file_dag) reuses: Cell<CheckCount>,
}
impl RelationOwner {
    pub(in crate::application::file_dag) fn derived(&self) -> &super::derived::owner::DerivedOwner {
        &self.derived
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn derived_mut(
        &mut self,
    ) -> &mut super::derived::owner::DerivedOwner {
        &mut self.derived
    }
    pub(in crate::application::file_dag) fn generation_guard(&self) -> Arc<()> {
        Arc::clone(&self.generation.borrow())
    }
    pub(in crate::application::file_dag) fn generation_liveness(&self) -> Arc<GenerationLease> {
        Arc::clone(&self.generation_live.borrow())
    }
    pub(in crate::application::file_dag) fn matches_generation(
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
    ) -> Self {
        Self::with_limits(ReuseEligibility::from(permit), views, semantic)
    }
    #[cfg(test)]
    pub(in crate::application) fn new(
        stable: ReuseEligibility,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic)
    }
    #[cfg(test)]
    pub(in crate::application) fn for_request(
        stable: ReuseEligibility,
        views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, views, semantic)
    }
    fn with_limits(
        stable: ReuseEligibility,
        max_views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::types::DerivedAccess>,
    ) -> Self {
        let max_derived = super::derived::owner::types::EntryLimit(
            max_views
                .0
                .saturating_mul(ryo_derived::domain::request::types::DerivedKind::ALL.len()),
        );
        Self {
            #[cfg(test)]
            identity_checks: Cell::new(CheckCount(0)),
            derived: super::derived::owner::DerivedOwner::new(max_derived, semantic),
            next_population: Cell::new(PopulationId(0)),
            producer_populations: RefCell::new(HashMap::new()),
            generation: RefCell::new(Arc::new(())),
            generation_live: RefCell::new(Arc::new(GenerationLease::new())),
            stable,
            sources: RefCell::new(HashMap::new()),
            max_views,
            views: Cell::new(PinnedViewCount(0)),
            checks: Cell::new(CheckCount(0)),
            reuses: Cell::new(CheckCount(0)),
        }
    }
    pub(super) fn stable(&self) -> ReuseEligibility {
        self.stable
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub(super) fn release_population(&self) {
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
    left: BorrowedRows<'_>,
    right: BorrowedRows<'_>,
) -> Sameness {
    identity::same(left, right)
}
