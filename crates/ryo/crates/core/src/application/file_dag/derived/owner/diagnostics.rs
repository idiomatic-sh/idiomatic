//! Read-only diagnostic reporting stays inside the private resource owner.
use super::*;
use ryo_relations::domain::relations::types::FileId;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn report(&self, file: FileId) {
        if let Some(indexes) = self.backend.borrow().as_ref() {
            let state = self.state.borrow();
            tracing::trace!(target: "ryo::diagnostic::graph", event = "file_graphs", file = file.0,
                builds = indexes.graph_counts.builds, reuses = indexes.graph_counts.reuses, refusals = indexes.graph_counts.refusals,
                roots = indexes.graph_counts.roots, traversals = indexes.graph_counts.traversals, memo_hits = indexes.graph_counts.memo_hits,
                vertices_expanded = indexes.graph_counts.expanded, edges_examined = indexes.graph_counts.examined,
                nearest_early_stops = indexes.graph_counts.early_stops, forest_hits = indexes.graph_counts.forest_hits,
                graph_bytes = indexes.graph_bytes, memo_bytes = indexes.memo_bytes, graph_enabled = indexes.graph_enabled,
                cache_bytes = indexes.bytes, cache_limit = indexes.limit, cache_peak_bytes = indexes.graph_counts.peak_bytes,
                reclaimed_indexes = indexes.graph_counts.reclaimed_indexes, reclaimed_selections = indexes.graph_counts.reclaimed_selections,
                reclaimed_bytes = indexes.graph_counts.reclaimed_bytes, pinned_refusals = indexes.graph_counts.pinned_refusals, index_refusals = indexes.refusals,
                selector_refusals = indexes.selection_counts.refusals);
            tracing::trace!(target: "ryo::diagnostic::selection", event = "file_selections", file = file.0,
                source_rows_scanned = indexes.selection_counts.scanned, indexed_candidates = indexes.selection_counts.candidates,
                predicate_cells_decoded = indexes.selection_counts.predicate_cells, output_cells_decoded = indexes.selection_counts.output_cells,
                selections_built = indexes.selection_counts.builds, selections_reused = indexes.selection_counts.reuses,
                selections_refused = indexes.selection_counts.refusals, cache_bytes = indexes.bytes,
                collected_cell_bytes = indexes.selection_counts.collected_cells.saturating_mul(size_of::<ryo_relations::domain::relations::types::Value>() as u64),
                compact_id_bytes = self.ledger.borrow().snapshot().optional_units.saturating_mul(size_of::<u32>()));
            tracing::trace!(target: "ryo::diagnostic::stored_join", event = "file_indexes", file = file.0,
                entries = state.entries.values().filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some()).count(), eligible_relations = indexes.eligible.len(),
                retained_bytes = indexes.bytes, physical_builds = indexes.builds,
                reuses = indexes.reuses, refusals = indexes.refusals);
            if let Some(entries) = &indexes.selection_histogram {
                for entry in entries {
                    let retained = state.entries.values().find_map(|candidate| {
                        let ViewRequest::PredicateMatches(_selector) = &candidate._plan.request
                        else {
                            return None;
                        };
                        if !super::super::request::identity::same_request(
                            &candidate._plan.request,
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
                    tracing::trace!(target: "ryo::diagnostic::selection_detail", event = "selector_histogram", file = file.0,
                        selector_id = std::ptr::from_ref(entry.selector.as_ref()).addr(),
                        relation = ?entry.selector.relation, comparisons = ?entry.selector.identity.comparisons,
                        bindings = ?entry.selector.identity.bindings, columns = ?entry.selector.columns,
                        distinct = entry.selector.distinct, calls = entry.calls, builds = entry.builds,
                        reuses = entry.reuses, errors = entry.errors, matched_rows = entry.matched_rows,
                        full_scan_rows = entry.full_scan_rows, index_build_rows = entry.index_build_rows,
                        indexed_candidates = entry.indexed_candidates, predicate_cells = entry.predicate_cells,
                        no_discriminator = entry.paths[0], not_repeated = entry.paths[1],
                        small_population = entry.paths[2], dense_sample = entry.paths[3],
                        index_budget_refused = entry.paths[4], index_built = entry.paths[5],
                        index_reused = entry.paths[6], fanout_built = entry.paths[7],
                        fanout_reused = entry.paths[8], admission_not_repeated = entry.admission[0],
                        admission_allocation_refused = entry.admission[1],
                        admission_budget_refused = entry.admission[2], admission_retained = entry.admission[3],
                        retained_rows = retained.as_ref().map_or(0, |result| result.as_slice().len()),
                        retained_slots = retained.as_ref().map_or(0, |result| result.capacity()));
                }
            }
        }
    }
}
