//! The `relations` application operation.

use super::super::derived::population::service::PopulationId;
use super::generation::service::GenerationLease;
use super::handle::service::PublishedInput;
use super::number::service::service::AdmittedNumber;
use super::publication::types::SourceDescriptor;
use super::publication::types::{ProducerKey, ProducerPopulation};
use super::types::{CheckCount, PinnedViewCount, ReuseEligibility, Sameness};
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::{CellKind, Relation};
use std::{
    cell::{Cell, RefCell},
    sync::Arc,
};

/// The relation owner of one file generation: it publishes relations, admits borrowed inputs and issues the generation they are bound to.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
/// fn named(_: Option<&RelationOwner>) {}
/// ```
pub struct RelationOwner {
    pub(super) derived: super::super::derived::owner::service::DerivedOwner,
    pub(super) next_population: Cell<PopulationId>,
    pub(super) producer_populations: RefCell<FxHashMap<ProducerKey, ProducerPopulation>>,
    /// The lease of the resident population: its allocation is the generation, replaced on each release.
    pub(super) generation: RefCell<Arc<GenerationLease>>,
    pub(super) stable: ReuseEligibility,
    pub(super) sources: RefCell<FxHashMap<Relation, SourceViews>>,
    pub(super) max_views: PinnedViewCount,
    pub(super) views: Cell<PinnedViewCount>,
    pub(super) checks: Cell<CheckCount>,
    pub(super) reuses: Cell<CheckCount>,
    /// The route profile of the scope that opened this owner.
    pub(super) routes: super::super::route_counts::profile::service::RouteProfile,
}

impl RelationOwner {
    /// The derived-view owner of this generation.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
    /// let _ = RelationOwner::derived;
    /// ```
    pub fn derived(&self) -> &super::super::derived::owner::service::DerivedOwner {
        &self.derived
    }
    /// The route profile the opening scope handed down, for this owner's work to record into.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
    /// let _ = RelationOwner::routes;
    /// ```
    pub fn routes(&self) -> &super::super::route_counts::profile::service::RouteProfile {
        &self.routes
    }
    /// The admitted inputs this owner has inspected, built or reused: one per input a view checked against the resident population
    /// and found current. A refused inspection (a stale or mismatched input) is not counted.
    #[cfg(test)]
    pub(crate) fn admitted_inspections(&self) -> CheckCount {
        CheckCount(self.checks.get().0 + self.reuses.get().0)
    }
    #[cfg(test)]
    pub(crate) fn derived_mut(
        &mut self,
    ) -> &mut super::super::derived::owner::service::DerivedOwner {
        &mut self.derived
    }
    /// The generation of the resident population, for a proof to keep and present back.
    /// The resident population's generation: the lease the owner issued for it.
    pub(crate) fn current_generation(&self) -> Arc<GenerationLease> {
        Arc::clone(&self.generation.borrow())
    }
    /// Whether `generation` is the resident population's generation.
    pub(crate) fn matches_generation(&self, generation: &Arc<GenerationLease>) -> Sameness {
        GenerationLease::same(generation, &self.generation.borrow())
    }

    pub(crate) fn for_scope(
        permit: super::super::service::permit::service::ScopePermit,
        views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
        routes: super::super::route_counts::profile::service::RouteProfile,
    ) -> Self {
        Self::with_limits(ReuseEligibility::from(permit), views, semantic, routes)
    }
    /// An owner over `semantic` with the default pin limit and no route recording, for the proofs of the store and the engine.
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn new(
        stable: ReuseEligibility,
        semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, Default::default())
    }
    /// An owner whose work records into `routes`.
    #[cfg(test)]
    pub(crate) fn recording(
        stable: ReuseEligibility,
        semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
        routes: super::super::route_counts::profile::service::RouteProfile,
    ) -> Self {
        Self::with_limits(stable, PinnedViewCount(1024), semantic, routes)
    }
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn for_request(
        stable: ReuseEligibility,
        views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
    ) -> Self {
        Self::with_limits(stable, views, semantic, Default::default())
    }
    fn with_limits(
        stable: ReuseEligibility,
        max_views: PinnedViewCount,
        semantic: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
        routes: super::super::route_counts::profile::service::RouteProfile,
    ) -> Self {
        let max_derived = super::super::derived::owner::types::EntryLimit(
            max_views
                .0
                .saturating_mul(ryo_derived::domain::request::types::DerivedKind::ALL.len()),
        );
        Self {
            derived: super::super::derived::owner::service::DerivedOwner::new(
                max_derived,
                semantic,
                routes.clone(),
            ),
            next_population: Cell::new(PopulationId(0)),
            producer_populations: RefCell::new(FxHashMap::default()),
            generation: RefCell::new(Arc::new(GenerationLease::new())),
            stable,
            sources: RefCell::new(FxHashMap::default()),
            max_views,
            views: Cell::new(PinnedViewCount(0)),
            checks: Cell::new(CheckCount(0)),
            reuses: Cell::new(CheckCount(0)),
            routes,
        }
    }
    /// A store over `derived` with the default pin limit and no route recording, for the proofs of the store and the engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::fixtures::program_derived_port;
    /// use ryo_execution_store::application::file_dag::relations::{types::ReuseEligibility, service::RelationOwner};
    /// let owner = RelationOwner::fixture(ReuseEligibility::Eligible, program_derived_port());
    /// assert_eq!(owner.stable(), ReuseEligibility::Eligible);
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    #[must_use]
    pub fn fixture(
        reuse: ReuseEligibility,
        derived: impl Into<crate::application::file_dag::derived::owner::service::DerivedAccess>,
    ) -> Self {
        Self::with_limits(reuse, PinnedViewCount(1024), derived, Default::default())
    }
    /// Whether this owner's publications may be reused across invocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::relations::service::RelationOwner;
    /// let _ = RelationOwner::stable;
    /// ```
    #[cfg(any(test, feature = "fixtures"))]
    pub fn stable(&self) -> ReuseEligibility {
        self.stable
    }
    /// Explicitly ends the immutable resident-population lease before release or refill.
    pub(crate) fn release_population(&self) {
        self.derived.clear();
        self.producer_populations.borrow_mut().clear();
        self.sources.borrow_mut().clear();
        self.views.set(PinnedViewCount(0));
        self.generation.borrow().invalidate();
        *self.generation.borrow_mut() = Arc::new(GenerationLease::new());
    }
}

impl super::interface::sealed::Sealed for RelationOwner {}

impl super::interface::RelationsPort for RelationOwner {
    fn generation(&self) -> super::generation::service::ResidentGeneration {
        super::generation::service::ResidentGeneration(self.current_generation())
    }
    fn publish(
        &self,
        body: super::publication::types::PublicationBody<'_>,
    ) -> Result<super::handle::service::Publication, OperatingError> {
        use super::handle::service::{Publication, PublicationHold};
        use super::publication::types::PublicationBody;
        Ok(Publication(match body {
            PublicationBody::Source { relation, rows } => {
                PublicationHold::Relation(self.source(relation, rows)?)
            }
            PublicationBody::Compact(table) => {
                PublicationHold::Relation(self.source_compact(table)?)
            }
            PublicationBody::Shared(table) => {
                PublicationHold::Relation(self.source_shared_computed(table)?)
            }
            PublicationBody::Computed(table) => PublicationHold::Relation(self.computed(table)?),
            PublicationBody::Packed { words, arenas } => PublicationHold::Packed {
                _lease: self.packed_publication(words, arenas)?,
            },
        }))
    }
    fn population(&self) -> Result<PopulationId, OperatingError> {
        self.project_identity()
    }
    fn admit<'a>(
        &self,
        publication: &super::handle::service::Publication,
        rows: RepresentationInput<'a>,
    ) -> Result<Result<super::admission::service::BorrowedInput<'a>, OperatingError>, OperatingError>
    {
        self.inspect_available(publication.input()?.as_ref(), rows)
    }
    fn admit_number<'a>(
        &self,
        publication: &'a super::handle::service::Publication,
        producer: super::types::ProducerId,
        required: &ryo_execution_domain::domain::self_join_recogniser::types::NumberRequirement,
        rows: RepresentationInput<'_>,
    ) -> Result<Option<AdmittedNumber<'a>>, OperatingError> {
        RelationOwner::admit_number(self, publication.input()?, producer, required, rows)
    }
    fn rows<'r, 'f: 'r>(
        &self,
        publication: &'r super::handle::service::Publication,
        read: &dyn Fn(Relation) -> Result<Rows<'f>, OperatingError>,
    ) -> Result<Rows<'r>, OperatingError> {
        self.resolve(publication.input()?.as_ref(), read)
    }
    fn normalize<'r, 'f: 'r>(
        &self,
        publication: &'r super::handle::service::Publication,
        rows: Rows<'f>,
    ) -> Result<Rows<'r>, OperatingError> {
        RelationOwner::normalize(self, publication.input()?, rows)
    }
    fn kinds<'a>(
        &self,
        publication: &'a super::handle::service::Publication,
    ) -> Result<&'a [CellKind], OperatingError> {
        self.publication_kinds(publication.input()?)
    }
    fn bind_producer(
        &self,
        scope: super::publication::types::ProducerScope,
        producer: super::types::ProducerId,
        publication: &super::handle::service::Publication,
    ) -> Result<(), OperatingError> {
        RelationOwner::bind_producer(self, scope, producer, publication.input()?.as_ref())
    }
    fn pin(&self, publication: &super::handle::service::Publication) {
        if let Ok(input) = publication.input() {
            self.pin_derived(input.as_ref());
        }
    }
    fn retire_producer(
        &self,
        scope: super::publication::types::ProducerScope,
        producer: super::types::ProducerId,
    ) {
        self.release_derived_producer(scope, producer);
    }
    fn release(&self) {
        self.release_population();
    }
}

pub(crate) fn same_admitted_rows(
    left: RepresentationInput<'_>,
    right: RepresentationInput<'_>,
) -> Sameness {
    super::identity::service::same(left, right)
}

pub(super) struct SourceViews {
    pub(crate) population: SourceDescriptor,
    pub(crate) canonical_population: PopulationId,
    pub(crate) views: FxHashMap<SourceDescriptor, PublishedInput>,
}
