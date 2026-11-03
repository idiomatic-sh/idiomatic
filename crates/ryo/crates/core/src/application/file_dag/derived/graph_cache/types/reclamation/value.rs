//! Preflight graph priority without evicting live consumers or recrediting retained metadata.

use crate::application::file_dag::derived::backend::{CacheBackend, Entry};
use crate::application::file_dag::derived::indexes::index_bytes;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Inline Entry metadata remains charged through the vector's unchanged capacity.
pub(in crate::application::file_dag::derived) fn index_payload(entry: &Entry) -> Option<usize> {
    index_bytes(&entry.index)?.checked_add(
        entry
            .columns
            .capacity()
            .checked_mul(size_of::<ColumnIndex>())?,
    )
}
