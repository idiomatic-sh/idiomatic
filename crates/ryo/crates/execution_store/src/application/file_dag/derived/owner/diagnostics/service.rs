//! The `diagnostics` application operation.

use ryo_derived::domain::request::types::DerivedKind;

use super::super::service::*;

use crate::application::file_dag::derived::constants;
use ryo_relations::domain::relations::types::FileId;

impl DerivedOwner {
    /// Reports the owner's counts for `file` to the diagnostic trace.
    pub fn report(&self, file: FileId) {
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
                graph_bytes = indexes.graph_bytes.0, memo_bytes = indexes.memo_bytes.0, graph_enabled = ?indexes.graph_enabled,
                cache_bytes = indexes.bytes.0, cache_limit = indexes.limit.0, cache_peak_bytes = indexes.graph_counts.peak_bytes.0,
                reclaimed_indexes = indexes.graph_counts.reclaimed_indexes.0,
                reclaimed_bytes = indexes.graph_counts.reclaimed_bytes.0, pinned_refusals = indexes.graph_counts.pinned_refusals.0, index_refusals = indexes.refusals.0);
            tracing::trace!(target: constants::TARGET_STORED_JOIN, event = constants::EVENT_FILE_INDEXES, file = file.0,
                entries = state.entries.values().filter(|entry| entry._plan.kind == DerivedKind::Equality && entry.retained.is_some()).count(), eligible_relations = indexes.eligible.len(),
                retained_bytes = indexes.bytes.0, physical_builds = indexes.builds.0,
                reuses = indexes.reuses.0, refusals = indexes.refusals.0);
        }
    }
}
