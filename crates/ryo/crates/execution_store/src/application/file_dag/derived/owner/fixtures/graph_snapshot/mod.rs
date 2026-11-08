//! The observed state of the owner and its cache after a fixture's actions.
use super::graph::GraphFixture;
use crate::application::file_dag::derived::owner::*;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What the owner counted, as plain numbers for a test to compare.
#[derive(Debug)]
pub struct SnapshotCounts {
    pub reclaimed_indexes: u64,
    pub reclaimed_bytes: u64,
    pub pinned_refusals: u64,
    pub builds: u64,
    pub reuses: u64,
    pub refusals: u64,
    pub traversals: u64,
    pub memo_hits: u64,
    pub examined: u64,
    pub forest_hits: u64,
}

/// What the fixture's owner holds, as plain numbers for a test to compare.
pub struct Snapshot {
    pub counts: SnapshotCounts,
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
}
impl GraphFixture {
    /// What the fixture's owner holds now.
    pub fn snapshot(&self) -> Snapshot {
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
            counts: SnapshotCounts {
                reclaimed_indexes: c.reclaimed_indexes.0,
                reclaimed_bytes: c.reclaimed_bytes.0,
                pinned_refusals: c.pinned_refusals.0,
                builds: c.builds.0,
                reuses: c.reuses.0,
                refusals: c.refusals.0,
                traversals: c.traversals.0,
                memo_hits: c.memo_hits.0,
                examined: c.examined.0,
                forest_hits: c.forest_hits.0,
            },
            bytes: cache.bytes.0,
            limit: cache.limit.0,
            memo_bytes: cache.memo_bytes.0,
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
                    _ => panic!("filtered equality entry"),
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
        }
    }
}
