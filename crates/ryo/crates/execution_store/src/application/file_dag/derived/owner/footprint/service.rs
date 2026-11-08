//! Physical allocation facts are supplied by the immutable capability payload.
use crate::application::file_dag::derived::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::acquisition::semantics::storage::DerivedStorage;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::sync::Arc;
pub(crate) fn storage_bytes(
    storage: &Arc<DerivedStorage>,
) -> Result<ByteCount, OperatingError> {
    DerivedStorage::backings(storage)?
        .iter()
        .try_fold(ByteCount(0), |total, backing| {
            total
                .checked_add(backing.bytes)
                .ok_or(OperatingError::ShapeMismatch)
        })
}
