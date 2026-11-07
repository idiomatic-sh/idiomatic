//! The observed state of the owner and its cache after a fixture's actions.
use super::graph::GraphFixture;
use crate::application::file_dag::derived::owner::*;
use crate::domain::stored_graph::types::GraphCounts;
use ryo_values::domain::plan_parts::types::ColumnIndex;

pub(in crate::application::file_dag) struct Snapshot {
    pub counts: GraphCounts,
    pub bytes: usize,
    pub limit: usize,
    pub memo_bytes: usize,
    pub graphs: usize,
    pub domains: usize,
    pub nearest: usize,
    pub indexes: usize,
    pub owner_entries_capacity: usize,
    pub index_payloads: Vec<usize>,
    pub index_columns: Vec<Vec<ColumnIndex>>,
    pub required_bytes: usize,
    pub optional_bytes: usize,
    pub unique_bytes: usize,
    pub overlap_bytes: usize,
    pub graph_backings: usize,
    pub retained_view_bytes: usize,
    pub selections: usize,
    pub retained_positions: usize,
}
impl GraphFixture {
    pub(in crate::application::file_dag) fn snapshot(&self) -> Snapshot {
        let backend = self.owner.derived().backend.borrow();
        let cache = backend.as_ref().unwrap();
        let c = &cache.graph_counts;
        let ledger = self.owner.derived().ledger.borrow();
        let s = ledger.snapshot();
        let state = self.owner.derived().state.borrow();
        let mut indexes: Vec<_> = state
            .entries
            .values()
            .filter(|entry| {
                entry.retained.is_some()
                    && matches!(entry._plan.request, ViewRequest::Equality { .. })
            })
            .collect();
        indexes.sort_by_key(|entry| entry.admission);
        Snapshot {
            counts: GraphCounts {
                reclaimed_indexes: c.reclaimed_indexes,
                reclaimed_selections: c.reclaimed_selections,
                reclaimed_bytes: c.reclaimed_bytes,
                pinned_refusals: c.pinned_refusals,
                peak_bytes: c.peak_bytes,
                builds: c.builds,
                reuses: c.reuses,
                refusals: c.refusals,
                roots: c.roots,
                traversals: c.traversals,
                memo_hits: c.memo_hits,
                expanded: c.expanded,
                examined: c.examined,
                early_stops: c.early_stops,
                forest_hits: c.forest_hits,
            },
            bytes: cache.bytes,
            limit: cache.limit,
            memo_bytes: cache.memo_bytes,
            graphs: cache.graphs.len(),
            domains: cache.graph_domains.len(),
            nearest: cache.nearest.len(),
            indexes: indexes.len(),
            owner_entries_capacity: state.entries.capacity(),
            index_payloads: indexes.iter().map(|entry| entry.bytes.0).collect(),
            index_columns: indexes
                .iter()
                .map(|entry| match &entry._plan.request {
                    ViewRequest::Equality { columns } => columns.clone(),
                    _ => unreachable!("filtered equality entry"),
                })
                .collect(),
            required_bytes: s.required_bytes.0,
            optional_bytes: s.view_optional_bytes.0,
            unique_bytes: s.unique_bytes.0,
            overlap_bytes: s.overlap_bytes.0,
            graph_backings: ledger
                .backing_count(crate::application::file_dag::derived::ledger::types::BackingTag(9))
                .0,
            retained_view_bytes: self.owner.derived().state.borrow().retained_bytes.0,
            selections: self.owner.derived().state.borrow().retained_predicates.0,
            retained_positions: s.optional_units.0,
        }
    }
}
