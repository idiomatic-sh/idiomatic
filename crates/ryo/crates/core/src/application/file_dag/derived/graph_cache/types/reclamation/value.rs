//! Preflight graph priority without evicting live consumers or recrediting retained metadata.

use crate::application::file_dag::derived::graph_cache::service::room;
use crate::application::file_dag::derived::indexes::index_bytes;
use crate::domain::stored_graph::types::footprint::value::{arc_bytes, vector};
use crate::domain::stored_indexes::types::{Entry, FileIndexes};
use crate::domain::stored_selection::types::Matches;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use std::sync::Arc;

/// One generation's ordinary payload may yield to a newly demanded adjacency.
pub(crate) struct GraphAdmission<'a> {
    /// Shared total byte owner; every surviving capacity remains charged.
    cache: &'a mut FileIndexes,
}

impl<'a> GraphAdmission<'a> {
    /// The new graph already pins any selected source row IDs before this borrow.
    pub(crate) fn new(cache: &'a mut FileIndexes) -> Self {
        Self { cache }
    }

    /// Reclaim in original admission order only after proving the complete graph can fit.
    pub(crate) fn prepare(&mut self, cost: usize) -> bool {
        if room(self.cache, cost).is_some() {
            return true;
        }
        let Some(required) = self
            .cache
            .bytes
            .checked_add(cost)
            .and_then(|total| total.checked_sub(self.cache.limit))
        else {
            return false;
        };
        let mut available = 0_usize;
        let mut pinned = false;
        for entry in &self.cache.entries {
            if Arc::strong_count(&entry.index) == 1 {
                available = available.saturating_add(index_payload(entry).unwrap_or(0));
            } else {
                pinned = true;
            }
        }
        for entry in &self.cache.selections {
            if Arc::strong_count(&entry.rows) == 1 {
                available = available.saturating_add(selection_payload(entry).unwrap_or(0));
            } else {
                pinned = true;
            }
        }
        if available < required {
            self.cache.graph_counts.pinned_refusals += u64::from(pinned);
            return false;
        }
        let mut remaining = required;
        let mut released = 0_usize;
        let mut indexes = 0_u64;
        let mut selections = 0_u64;
        self.cache.entries.retain(|entry| {
            if remaining != 0
                && Arc::strong_count(&entry.index) == 1
                && let Some(bytes) = index_payload(entry)
            {
                remaining = remaining.saturating_sub(bytes);
                released += bytes;
                indexes += 1;
                return false;
            }
            true
        });
        self.cache.selections.retain(|entry| {
            if remaining != 0
                && Arc::strong_count(&entry.rows) == 1
                && let Some(bytes) = selection_payload(entry)
            {
                remaining = remaining.saturating_sub(bytes);
                released += bytes;
                selections += 1;
                return false;
            }
            true
        });
        self.cache.bytes -= released;
        self.cache.graph_counts.reclaimed_indexes += indexes;
        self.cache.graph_counts.reclaimed_selections += selections;
        self.cache.graph_counts.reclaimed_bytes += released as u64;
        room(self.cache, cost).is_some()
    }
}

/// Inline Entry metadata remains charged through the vector's unchanged capacity.
pub(crate) fn index_payload(entry: &Entry) -> Option<usize> {
    index_bytes(&entry.index)?.checked_add(
        entry
            .columns
            .capacity()
            .checked_mul(size_of::<ColumnIndex>())?,
    )
}

/// A dormant match vector has no other generation, graph, domain or live view retaining its Arc.
pub(crate) fn selection_payload(entry: &Matches) -> Option<usize> {
    arc_bytes::<Vec<u32>>().checked_add(vector(&entry.rows))
}
