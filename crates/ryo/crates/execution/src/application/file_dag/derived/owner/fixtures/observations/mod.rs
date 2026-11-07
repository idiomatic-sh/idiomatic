//! Test-only observations of actual owner backing and selection state.
use crate::application::file_dag::derived::owner::*;
use ryo_relations::domain::relations::types::Relation;
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::application::file_dag) struct IndexObservation {
    pub available: bool,
}
impl DerivedOwner {
    pub(in crate::application::file_dag) fn scoped_observation(
        &self,
    ) -> (usize, Weak<DerivedStorage>) {
        (
            self.next_scope.get().0,
            self.last_scoped_storage.borrow().clone(),
        )
    }

    pub(crate) fn resource_fixture_counts(
        &self,
    ) -> crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
        let backend = self.backend.borrow();
        let cache = backend.as_ref().expect("configured actual owner");
        crate::application::file_dag::derived::owner::fixtures::resources::Snapshot {
            graph_builds: cache.graph_counts.builds.0,
            graph_reuses: cache.graph_counts.reuses.0,
            graph_early_stops: cache.graph_counts.early_stops.0,
        }
    }
    pub(in crate::application::file_dag) fn index_observation(&self) -> IndexObservation {
        self.backend
            .borrow()
            .as_ref()
            .map_or_else(IndexObservation::default, |_| IndexObservation {
                available: true,
            })
    }
    pub(in crate::application::file_dag) fn relation_eligible(&self, relation: Relation) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.eligible.contains(&relation))
    }

    pub(in crate::application::file_dag::derived::owner) fn retained_index_count(&self) -> usize {
        self.state
            .borrow()
            .entries
            .values()
            .filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some())
            .count()
    }
    pub(in crate::application::file_dag) fn no_invocation_index_handles(&self) -> bool {
        self.state.borrow().entries.values().all(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return true;
            }
            let owned =
                usize::from(entry.retained.is_some()) + usize::from(entry.required.is_some());
            entry.weak.strong_count() == owned
        })
    }

    pub(in crate::application::file_dag::derived::owner) fn first_index_weak(
        &self,
    ) -> Option<Weak<ryo_derived::domain::contract::types::KeyIndex>> {
        self.state.borrow().entries.values().find_map(|entry| {
            if entry._plan.kind != DerivedKind::Equality {
                return None;
            }
            let storage = entry.weak.upgrade()?;
            storage
                .payload()
                .share_equality()
                .map(|index| Arc::downgrade(&index))
        })
    }
    pub(in crate::application::file_dag) fn shared_index_backings(&self) -> usize {
        self.ledger
            .borrow()
            .backing_count(crate::application::file_dag::derived::ledger::types::BackingTag(11))
            .0
    }
    pub(in crate::application::file_dag) fn publication_ownership_bytes(
        &self,
    ) -> (usize, usize, usize) {
        let state = self.ledger.borrow().snapshot();
        (
            state.required_bytes.0,
            state.optional_bytes.0,
            state.unique_bytes.0,
        )
    }
    /// The observation follows one qualified producer's admitted populations.
    /// Ambiguous matching requirements are refused rather than picking a view.
    pub(in crate::application::file_dag) fn population_view_witness(
        &self,
        populations: &[PopulationId],
        kind: DerivedKind,
    ) -> Result<
        Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>,
        OperatingError,
    > {
        let state = self.state.borrow();
        let mut matching = state.entries.iter().filter(|(key, entry)| {
            entry._plan.kind == kind
                && key.populations[..usize::from(key.len.0)]
                    .iter()
                    .any(|population| populations.contains(population))
        });
        let witness = matching.next().map(|(_, entry)| {
            crate::application::file_dag::derived::owner::witness::service::BackingWitness::new(
                entry.weak.clone(),
            )
        });
        if matching.next().is_some() {
            return Err(OperatingError::WrongOperation);
        }
        Ok(witness)
    }
    pub(in crate::application::file_dag) fn view_witness(
        &self,
        kind: DerivedKind,
    ) -> Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>
    {
        self.state
            .borrow()
            .entries
            .values()
            .find(|entry| entry._plan.kind == kind)
            .map(|entry| {
                crate::application::file_dag::derived::owner::witness::service::BackingWitness::new(
                    entry.weak.clone(),
                )
            })
    }
}

use crate::application::file_dag::derived::owner::selection::fixtures::{
    SelectionCounters, SelectionSnapshot,
};
use crate::domain::stored_selection::types::{CandidateRepeat, Selector};
impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner) fn fixture_limit_to_metadata(&self) {
        if let Some(cache) = self.backend.borrow_mut().as_mut() {
            cache.limit = cache.bytes;
        }
    }
    pub(in crate::application::file_dag::derived::owner) fn fixture_ineligible(&self) {
        if let Some(cache) = self.backend.borrow_mut().as_mut() {
            cache.eligible.clear();
        }
    }
    pub(in crate::application::file_dag::derived::owner) fn fixture_corrupt_prepared_group_count(
        &self,
    ) {
        let mut backend = self.backend.borrow_mut();
        let cache = backend.as_mut().expect("selection fixture backend");
        let mut prepared = cache
            .selection_views
            .take()
            .expect("prepared selector views");
        Arc::get_mut(&mut prepared)
            .expect("unshared request fixture")
            .groups = crate::domain::derived_views::types::ViewGroupCount(0);
        crate::application::file_dag::derived::owner::selection::preparation::service::install_views(cache, prepared);
    }
    pub(in crate::application::file_dag::derived::owner) fn fixture_reserve_spare_selections(
        &self,
        extra: usize,
    ) {
        self.state
            .borrow_mut()
            .entries
            .try_reserve(extra)
            .expect("test spare metadata capacity");
    }
    pub(in crate::application::file_dag::derived::owner) fn fixture_snapshot(
        &self,
        descriptor: Option<&Arc<Selector>>,
    ) -> SelectionSnapshot {
        let backend = self.backend.borrow();
        let Some(cache) = backend.as_ref() else {
            return SelectionSnapshot::default();
        };
        let counts = &cache.selection_counts;
        let state = self.state.borrow();
        let mut rows = Vec::new();
        let mut predicates = Vec::new();
        let mut add_rows = |value: &Arc<Vec<u32>>| {
            let address = Arc::as_ptr(value) as usize;
            if !rows.iter().any(|(known, _)| *known == address) {
                rows.push((address, value.capacity()));
            }
        };
        for entry in state.entries.values() {
            let Some(retained) = &entry.retained else {
                continue;
            };
            if let Some(shared) = retained.storage.payload().share_positions() {
                add_rows(shared.backing());
                if entry._plan.kind == DerivedKind::PredicateMatches {
                    predicates.push(Arc::as_ptr(shared.backing()) as usize);
                }
            }
            if let ryo_derived::domain::contract::types::DerivedView::Postings(postings) =
                retained.storage.view().expect("valid retained payload")
            {
                for posting in postings {
                    let shared = retained
                        .storage
                        .payload()
                        .share_posting_positions(posting.text())
                        .expect("stored posting");
                    add_rows(shared.backing());
                }
            }
            if let Some(graph) = retained.storage.graph_identity() {
                if let Some(positions) = &graph.population.positions {
                    add_rows(&positions.0);
                }
            }
        }
        let fanouts: Vec<_> = state
            .entries
            .values()
            .filter(|entry| {
                entry.retained.is_some() && entry._plan.kind == DerivedKind::CandidatePostings
            })
            .collect();
        let capacity = rows.iter().map(|(_, capacity)| capacity).sum();
        let optional = self.ledger.borrow().snapshot();
        SelectionSnapshot {
            bytes: cache.bytes,
            limit: cache.limit,
            index_builds: cache.builds.0,
            entries: self.retained_index_count(),
            selections: predicates.len(),
            selections_capacity: state.entries.capacity(),
            owner_metadata_bytes: self.metadata_bytes().0,
            optional_bytes: optional.view_optional_bytes.0,
            retained_positions: optional.optional_units.0,
            first_retained_rows_address: predicates.first().copied(),
            retained_positions_capacity: capacity,
            fanouts: fanouts.len(),
            fanout_payload_bytes: fanouts.iter().map(|entry| entry.bytes.0).sum(),
            descriptor_address: cache.selector_descriptor.map(|address| address.0),
            prepared_descriptor_address: descriptor
                .and_then(|program| {
                    cache
                        .selection_views
                        .as_ref()?
                        .by_selector
                        .get(&(Arc::as_ptr(program) as usize))
                })
                .map(|view| Arc::as_ptr(view) as usize),
            prepared_views_address: cache
                .selection_views
                .as_ref()
                .map(|views| Arc::as_ptr(views) as usize),
            active_repetition_groups: cache.selector_repetition.len(),
            repetition_capacity: cache.selector_repetition.capacity(),
            prepared_selectors: cache
                .selection_views
                .as_ref()
                .map_or(0, |views| views.by_selector.len()),
            repeated_discriminator: cache
                .discriminator_columns
                .iter()
                .any(|column| column.repeat == CandidateRepeat::Repeated),
            counts: SelectionCounters {
                scanned: counts.scanned.0,
                candidates: counts.candidates.0,
                predicate_cells: counts.predicate_cells.0,
                builds: counts.builds.0,
                reuses: counts.reuses.0,
                refusals: counts.refusals.0,
            },
        }
    }
    pub(in crate::application::file_dag::derived::owner) fn fixture_install_views(
        &self,
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [ryo_relations::domain::relations::types::CellKind],
    ) -> Result<(), OperatingError> {
        let mut backend = self.backend.borrow_mut();
        let cache = backend.as_mut().ok_or(OperatingError::ShapeMismatch)?;
        cache.selectors = active;
        let prepared =
            crate::application::file_dag::derived::owner::selection::preparation::service::prepare_views_with_schemas(
                authored,
                &|_| kinds,
                (std::sync::Arc::new(
                    ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
                )
                    as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
            )?;
        crate::application::file_dag::derived::owner::selection::preparation::service::install_views(cache, prepared);
        crate::application::file_dag::derived::owner::index::service::candidate_columns(cache);
        drop(backend);
        Ok(())
    }
}
