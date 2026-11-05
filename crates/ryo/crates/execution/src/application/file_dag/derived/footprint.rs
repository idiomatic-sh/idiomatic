//! Physical allocation facts are supplied by the immutable capability payload.
use super::storage::DerivedStorage;
use crate::domain::operating::errors::OperatingError;
use std::sync::Arc;
pub(super) fn storage_bytes(storage: &Arc<DerivedStorage>) -> Result<usize, OperatingError> {
    DerivedStorage::backings(storage)?
        .iter()
        .try_fold(0_usize, |total, backing| {
            total
                .checked_add(backing.bytes)
                .ok_or(OperatingError::ShapeMismatch)
        })
}
