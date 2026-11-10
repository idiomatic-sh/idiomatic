//! The observed state of the owner and its cache after a fixture's actions.
use super::graph::GraphFixture;
#[cfg(test)]
use crate::application::file_dag::derived::owner::*;
#[cfg(test)]
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// What the owner counted, as plain numbers for a test to compare.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct SnapshotCounts {
    pub(crate) reclaimed_indexes: u64,
    pub(crate) reclaimed_bytes: u64,
    pub(crate) pinned_refusals: u64,
    pub(crate) builds: u64,
    pub(crate) reuses: u64,
    pub(crate) refusals: u64,
    pub(crate) traversals: u64,
    pub(crate) memo_hits: u64,
    pub(crate) examined: u64,
    pub(crate) forest_hits: u64,
}

/// What the fixture's owner holds, as plain numbers for a test to compare.
pub(crate) struct Snapshot {
    #[cfg(test)]
    pub(crate) counts: SnapshotCounts,
    #[cfg(test)]
    pub(crate) bytes: usize,
    #[cfg(test)]
    pub(crate) limit: usize,
    #[cfg(test)]
    pub(crate) memo_bytes: usize,
    #[cfg(test)]
    pub(crate) graphs: usize,
    #[cfg(test)]
    pub(crate) domains: usize,
    #[cfg(test)]
    pub(crate) nearest: usize,
    #[cfg(test)]
    pub(crate) indexes: usize,
    #[cfg(test)]
    pub(crate) owner_entries_capacity: usize,
    #[cfg(test)]
    pub(crate) index_payloads: Vec<usize>,
    #[cfg(test)]
    pub(crate) index_columns: Vec<Vec<ColumnIndex>>,
    pub(crate) required_bytes: usize,
    pub(crate) optional_bytes: usize,
    pub(crate) unique_bytes: usize,
    #[cfg(test)]
    pub(crate) overlap_bytes: usize,
    #[cfg(test)]
    pub(crate) graph_backings: usize,
    #[cfg(test)]
    pub(crate) retained_view_bytes: usize,
}
impl GraphFixture {
    /// What the fixture's owner holds now.
    pub(crate) fn snapshot(&self) -> Snapshot {
        #[cfg(test)]
        let backend = self.owner.derived().backend.borrow();
        #[cfg(test)]
        let cache = backend.as_ref().unwrap();
        #[cfg(test)]
        let c = &cache.graph_counts;
        let ledger = self.owner.derived().ledger.borrow();
        let s = ledger.snapshot();
        #[cfg(test)]
        let state = self.owner.derived().state.borrow();
        #[cfg(test)]
        let mut indexes: Vec<_> = state
            .entries
            .values()
            .filter(|entry| {
                entry.retained.is_some()
                    && matches!(entry._plan.request, ViewRequest::Equality { .. })
            })
            .collect();
        #[cfg(test)]
        indexes.sort_by_key(|entry| entry.admission);
        Snapshot {
            #[cfg(test)]
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
            #[cfg(test)]
            bytes: cache.bytes.0,
            #[cfg(test)]
            limit: cache.limit.0,
            #[cfg(test)]
            memo_bytes: cache.memo_bytes.0,
            #[cfg(test)]
            graphs: cache.graphs.len(),
            #[cfg(test)]
            domains: cache.graph_domains.len(),
            #[cfg(test)]
            nearest: cache.nearest.len(),
            #[cfg(test)]
            indexes: indexes.len(),
            #[cfg(test)]
            owner_entries_capacity: state.entries.capacity(),
            #[cfg(test)]
            index_payloads: indexes.iter().map(|entry| entry.bytes.0).collect(),
            #[cfg(test)]
            index_columns: indexes
                .iter()
                .map(|entry| {
                    let ViewRequest::Equality { columns } = &entry._plan.request else {
                        panic!("filtered equality entry");
                    };
                    columns.clone()
                })
                .collect(),
            required_bytes: s.required_bytes.0,
            optional_bytes: s.view_optional_bytes.0,
            unique_bytes: s.unique_bytes.0,
            #[cfg(test)]
            overlap_bytes: s.overlap_bytes.0,
            #[cfg(test)]
            graph_backings: ledger
                .backing_count(crate::application::file_dag::derived::ledger::types::BackingTag(9))
                .0,
            #[cfg(test)]
            retained_view_bytes: self.owner.derived().state.borrow().retained_bytes.0,
        }
    }
}
