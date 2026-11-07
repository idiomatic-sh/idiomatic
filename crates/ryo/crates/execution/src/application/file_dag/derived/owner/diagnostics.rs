//! Read-only diagnostic reporting stays inside the private resource owner.
use super::*;
use crate::application::file_dag::derived::constants;
use ryo_relations::domain::relations::types::FileId;
impl DerivedOwner {
    pub(super) fn report_selector_occupancy(&self, requesting: PopulationId) {
        let state = self.state.borrow();
        let ledger = self.ledger.borrow();
        let mut retained = state
            .entries
            .iter()
            .filter(|(_, entry)| entry.retained.is_some())
            .collect::<Vec<_>>();
        let releasable = retained
            .iter()
            .filter_map(|(_, entry)| {
                let value = entry.retained.as_ref()?;
                (entry.required.is_none()
                    && ledger.optional_payload_only(&entry.backings)
                        == crate::application::file_dag::derived::ledger::types::Dormancy::Dormant
                    && Arc::strong_count(&value.storage) == 1)
                    .then_some(&value._lease)
            })
            .collect::<Vec<_>>();
        tracing::trace!(target: constants::TARGET_SELECTION_DETAIL, event = constants::EVENT_SELECTOR_OCCUPANCY_SUMMARY,
            requesting = ?requesting, retained_entries = retained.len(), reclaimable_entries = releasable.len(),
            reclaimable_physical_credit = ?ledger.optional_release_credit(&releasable));
        retained
            .sort_unstable_by_key(|(_, entry)| (std::cmp::Reverse(entry.bytes), entry.admission));
        for (key, entry) in retained.into_iter().take(16) {
            let value = entry
                .retained
                .as_ref()
                .expect(constants::EXPECT_FILTERED_RETAINED);
            let populations = &key.populations[..usize::from(key.len.0)];
            let pins = populations
                .iter()
                .map(|id| (*id, state.pins.get(id).copied().unwrap_or_default()))
                .collect::<Vec<_>>();
            let mut request = format!("{:?}", entry._plan.request);
            request.truncate(
                request
                    .char_indices()
                    .nth(1024)
                    .map_or(request.len(), |(at, _)| at),
            );
            tracing::trace!(target: constants::TARGET_SELECTION_DETAIL, event = constants::EVENT_SELECTOR_OCCUPANCY_ENTRY,
                requesting = ?requesting, kind = ?entry._plan.kind, admission = ?entry.admission,
                populations = ?populations, population_pins = ?pins, logical_payload_bytes = entry.bytes.0,
                required_entry = entry.required.is_some(), storage_strong = Arc::strong_count(&value.storage),
                optional_payload_only = ?ledger.optional_payload_only(&entry.backings), backings = ?entry.backings,
                request = request);
        }
    }

    pub(in crate::application::file_dag) fn report(&self, file: FileId) {
        tracing::trace!(target: constants::TARGET_RESOURCE_ROUTES, event = constants::EVENT_OWNER_FOOTPRINT, file = file.0,
            required_bytes = self.working_bytes().0, peak_required_bytes = self.peak_working_bytes().0,
            retained_bytes = self.retained_bytes().0, required_backend_bytes = self.required_backend_overlap().0,
            peak_required_views = ?self.peak_required_views(), owned_bytes = self.owned_bytes().0,
            metadata_bytes = self.metadata_bytes().0, released_entries = self.state.borrow().released_entries.0);

        if let Some(indexes) = self.backend.borrow().as_ref() {
            let state = self.state.borrow();
            tracing::trace!(target: constants::TARGET_GRAPH, event = constants::EVENT_FILE_GRAPHS, file = file.0,
                builds = indexes.graph_counts.builds.0, reuses = indexes.graph_counts.reuses.0, refusals = indexes.graph_counts.refusals.0,
                roots = indexes.graph_counts.roots.0, traversals = indexes.graph_counts.traversals.0, memo_hits = indexes.graph_counts.memo_hits.0,
                vertices_expanded = indexes.graph_counts.expanded.0, edges_examined = indexes.graph_counts.examined.0,
                nearest_early_stops = indexes.graph_counts.early_stops.0, forest_hits = indexes.graph_counts.forest_hits.0,
                graph_bytes = indexes.graph_bytes, memo_bytes = indexes.memo_bytes, graph_enabled = ?indexes.graph_enabled,
                cache_bytes = indexes.bytes, cache_limit = indexes.limit, cache_peak_bytes = indexes.graph_counts.peak_bytes.0,
                reclaimed_indexes = indexes.graph_counts.reclaimed_indexes.0, reclaimed_selections = indexes.graph_counts.reclaimed_selections.0,
                reclaimed_bytes = indexes.graph_counts.reclaimed_bytes.0, pinned_refusals = indexes.graph_counts.pinned_refusals.0, index_refusals = indexes.refusals.0,
                selector_refusals = indexes.selection_counts.refusals.0);
            tracing::trace!(target: constants::TARGET_SELECTION, event = constants::EVENT_FILE_SELECTIONS, file = file.0,
                source_rows_scanned = indexes.selection_counts.scanned.0, indexed_candidates = indexes.selection_counts.candidates.0,
                predicate_cells_decoded = indexes.selection_counts.predicate_cells.0, output_cells_decoded = indexes.selection_counts.output_cells.0,
                selections_built = indexes.selection_counts.builds.0, selections_reused = indexes.selection_counts.reuses.0,
                selections_refused = indexes.selection_counts.refusals.0, cache_bytes = indexes.bytes,
                collected_cell_bytes = indexes.selection_counts.collected_cells.0.saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>() as u64),
                compact_id_bytes = self.ledger.borrow().snapshot().optional_units.0.saturating_mul(size_of::<u32>()));
            tracing::trace!(target: constants::TARGET_STORED_JOIN, event = constants::EVENT_FILE_INDEXES, file = file.0,
                entries = state.entries.values().filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some()).count(), eligible_relations = indexes.eligible.len(),
                retained_bytes = indexes.bytes, physical_builds = indexes.builds.0,
                reuses = indexes.reuses.0, refusals = indexes.refusals.0);
            if let Some(entries) = &indexes.selection_histogram {
                for entry in entries {
                    let retained = state.entries.values().find_map(|candidate| {
                        let ViewRequest::PredicateMatches(_selector) = &candidate._plan.request
                        else {
                            return None;
                        };
                        if super::super::request::identity::types::StaticRequest(
                            &candidate._plan.request,
                        ) != super::super::request::identity::types::StaticRequest(
                            &ViewRequest::PredicateMatches(Arc::clone(&entry.selector)),
                        ) {
                            return None;
                        }
                        candidate
                            .retained
                            .as_ref()?
                            .storage
                            .payload()
                            .share_positions()
                    });
                    tracing::trace!(target: constants::TARGET_SELECTION_DETAIL, event = constants::EVENT_SELECTOR_HISTOGRAM, file = file.0,
                        selector_id = std::ptr::from_ref(entry.selector.as_ref()).addr(),
                        relation = ?entry.selector.relation, comparisons = ?entry.selector.identity.comparisons,
                        bindings = ?entry.selector.identity.bindings, columns = ?entry.selector.columns,
                        distinct = entry.selector.distinct, calls = entry.calls.0, builds = entry.builds.0,
                        reuses = entry.reuses.0, errors = entry.errors.0, matched_rows = entry.matched_rows.0,
                        full_scan_rows = entry.full_scan_rows.0, index_build_rows = entry.index_build_rows.0,
                        indexed_candidates = entry.indexed_candidates.0, predicate_cells = entry.predicate_cells.0,
                        no_discriminator = entry.paths[0].0, not_repeated = entry.paths[1].0,
                        small_population = entry.paths[2].0, dense_sample = entry.paths[3].0,
                        index_budget_refused = entry.paths[4].0, index_built = entry.paths[5].0,
                        index_reused = entry.paths[6].0, fanout_built = entry.paths[7].0,
                        fanout_reused = entry.paths[8].0, admission_not_repeated = entry.admission[0].0,
                        admission_allocation_refused = entry.admission[1].0,
                        admission_budget_refused = entry.admission[2].0, admission_retained = entry.admission[3].0,
                        retained_rows = retained.as_ref().map_or(0, |result| result.as_slice().len()),
                        retained_slots = retained.as_ref().map_or(0, |result| result.capacity()));
                }
            }
        }
    }
}
