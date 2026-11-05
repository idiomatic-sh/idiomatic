//! Immutable policy answers and diagnostic values; no backend reference escapes.
use super::*;
use ryo_relations::domain::relations::types::{ColumnsView, Relation};
use ryo_values::domain::plan_parts::types::ColumnIndex;
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::application::file_dag) struct IndexObservation {
    pub available: bool,
}
impl DerivedOwner {
    #[cfg(test)]
    pub(in crate::application::file_dag) fn scoped_observation(
        &self,
    ) -> (usize, Weak<DerivedStorage>) {
        (
            self.next_scope.get(),
            self.last_scoped_storage.borrow().clone(),
        )
    }

    pub(in crate::application::file_dag) fn selector_work(
        &self,
    ) -> Result<crate::domain::operator::types::RowsFed, OperatingError> {
        let backend = self.backend.borrow();
        let backend = backend.as_ref().ok_or(OperatingError::NotRun)?;
        Ok(crate::domain::operator::types::RowsFed([
            backend.selection_counts.predicate_rows,
            0,
            0,
        ]))
    }

    #[cfg(test)]
    pub(crate) fn resource_fixture_counts(&self) -> super::resource_fixture::Snapshot {
        let backend = self.backend.borrow();
        let cache = backend.as_ref().expect("configured actual owner");
        super::resource_fixture::Snapshot {
            graph_builds: cache.graph_counts.builds,
            graph_reuses: cache.graph_counts.reuses,
            graph_early_stops: cache.graph_counts.early_stops,
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn index_observation(&self) -> IndexObservation {
        self.backend
            .borrow()
            .as_ref()
            .map_or_else(IndexObservation::default, |_| IndexObservation {
                available: true,
            })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn relation_eligible(&self, relation: Relation) -> bool {
        self.backend
            .borrow()
            .as_ref()
            .is_some_and(|cache| cache.eligible.contains(&relation))
    }
    pub(super) fn has_live_equality(&self, rows: ColumnsView<'_>, columns: &[ColumnIndex]) -> bool {
        let count = rows.columns.first().map_or(0, |column| column.0.len());
        self.state.borrow().entries.values().any(|entry| {
            matches!(&entry._plan.request, ViewRequest::Equality { columns: requested } if requested == columns)
                && entry._plan.schemas.first().is_some_and(|schema| schema == rows.kinds)
                && entry.weak.upgrade().is_some_and(|storage| {
                    matches!(storage.view(), Ok(ryo_derived::domain::contract::types::DerivedView::Equality(index)) if index.rows().len() == count)
                })
        })
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn retained_index_count(&self) -> usize {
        self.state
            .borrow()
            .entries
            .values()
            .filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some())
            .count()
    }
    #[cfg(test)]
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

    #[cfg(test)]
    pub(super) fn first_index_weak(
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
    #[cfg(test)]
    pub(in crate::application::file_dag) fn shared_index_backings(&self) -> usize {
        self.ledger.borrow().backing_count(11)
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn publication_ownership_bytes(
        &self,
    ) -> (usize, usize, usize) {
        let state = self.ledger.borrow().snapshot();
        (
            state.required_bytes,
            state.optional_bytes,
            state.unique_bytes,
        )
    }
    #[cfg(test)]
    pub(in crate::application::file_dag) fn view_weak(
        &self,
        kind: DerivedKind,
    ) -> Option<std::sync::Weak<DerivedStorage>> {
        self.state
            .borrow()
            .entries
            .values()
            .find(|entry| entry._plan.kind == kind)
            .map(|entry| entry.weak.clone())
    }
}

#[cfg(test)]
use crate::application::file_dag::derived::selection::fixtures::{
    SelectionCounters, SelectionSnapshot,
};
#[cfg(test)]
use crate::domain::stored_selection::types::Selector;
impl DerivedOwner {
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_limit_to_metadata(&self) {
        if let Some(cache) = self.backend.borrow_mut().as_mut() {
            cache.limit = cache.bytes;
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_ineligible(&self) {
        if let Some(cache) = self.backend.borrow_mut().as_mut() {
            cache.eligible.clear();
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_corrupt_prepared_group_count(&self) {
        let mut backend = self.backend.borrow_mut();
        let cache = backend.as_mut().expect("selection fixture backend");
        let mut prepared = cache
            .selection_views
            .take()
            .expect("prepared selector views");
        Arc::get_mut(&mut prepared)
            .expect("unshared request fixture")
            .groups = 0;
        super::super::selection::install_views(cache, prepared);
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_reserve_spare_selections(
        &self,
        extra: usize,
    ) {
        self.state
            .borrow_mut()
            .entries
            .try_reserve(extra)
            .expect("test spare metadata capacity");
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_snapshot(
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
                    add_rows(positions);
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
            index_builds: cache.builds,
            entries: self.retained_index_count(),
            selections: predicates.len(),
            selections_capacity: state.entries.capacity(),
            owner_metadata_bytes: self.metadata_bytes(),
            optional_bytes: optional.view_optional_bytes,
            retained_positions: optional.optional_units,
            first_retained_rows_address: predicates.first().copied(),
            retained_positions_capacity: capacity,
            fanouts: fanouts.len(),
            fanout_payload_bytes: fanouts.iter().map(|entry| entry.bytes).sum(),
            descriptor_address: cache.selector_descriptor,
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
            prepared_selectors: cache
                .selection_views
                .as_ref()
                .map_or(0, |views| views.by_selector.len()),
            repeated_discriminator: cache
                .discriminator_columns
                .iter()
                .any(|column| column.repeated),
            counts: SelectionCounters {
                scanned: counts.scanned,
                candidates: counts.candidates,
                predicate_cells: counts.predicate_cells,
                builds: counts.builds,
                reuses: counts.reuses,
                refusals: counts.refusals,
            },
        }
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::derived) fn fixture_install_views(
        &self,
        active: Vec<Arc<Selector>>,
        authored: impl IntoIterator<Item = Arc<Selector>>,
        kinds: &'static [ryo_relations::domain::relations::types::CellKind],
    ) -> Result<(), OperatingError> {
        let mut backend = self.backend.borrow_mut();
        let cache = backend.as_mut().ok_or(OperatingError::ShapeMismatch)?;
        cache.selectors = active;
        let prepared = super::super::selection::prepare_views_with_schemas(
            authored,
            &|_| kinds,
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        )?;
        super::super::selection::install_views(cache, prepared);
        super::super::indexes::candidate_columns(cache);
        drop(backend);
        Ok(())
    }
}
