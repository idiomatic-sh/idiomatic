//! Allocation identity and weak observation of shared positions and marks.
use super::types::{SharedMarks, SharedPositions, WeakPositions, WeakRows};
use crate::domain::allocation::types::{AllocationKey, ElementCapacity};
use std::sync::Arc;

/// The identity of the allocation the positions share.
#[must_use]
pub fn positions_key(positions: &SharedPositions) -> AllocationKey {
    AllocationKey(Arc::as_ptr(&positions.0) as usize)
}

/// The capacity of the allocation the positions share.
#[must_use]
pub fn positions_capacity(positions: &SharedPositions) -> ElementCapacity {
    ElementCapacity(positions.0.capacity())
}

/// A weak handle on the allocation the positions share.
#[must_use]
pub fn downgrade_positions(positions: &SharedPositions) -> WeakPositions {
    WeakPositions {
        allocation_key: positions_key(positions),
        capacity: positions_capacity(positions),
        rows: WeakRows(Arc::downgrade(&positions.0)),
    }
}

/// The positions, while some owner still holds them.
#[must_use]
pub fn upgrade_positions(weak: &WeakPositions) -> Option<SharedPositions> {
    weak.rows.0.upgrade().map(SharedPositions)
}

/// The identity of the allocation the marks share.
#[must_use]
pub fn marks_key(marks: &SharedMarks) -> AllocationKey {
    AllocationKey(Arc::as_ptr(&marks.0) as usize)
}

/// The capacity of the allocation the marks share.
#[must_use]
pub fn marks_capacity(marks: &SharedMarks) -> ElementCapacity {
    ElementCapacity(marks.0.capacity())
}
