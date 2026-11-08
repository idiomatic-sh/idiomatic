//! The replacement storage and the metadata bytes a collection growth costs.
use crate::application::file_dag::derived::ledger::types::ByteCount;

/// Replacement storage for a full collection, with the metadata bytes it adds.
pub(crate) struct Reserve<T> {
    pub replacement: Option<Vec<T>>,
    pub metadata: ByteCount,
}
