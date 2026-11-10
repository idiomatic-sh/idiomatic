//! Allocation identity and weak observation of shared positions and marks.
use super::types::SharedMarks;
use crate::domain::allocation::types::{AllocationKey, ElementCapacity};
use std::sync::Arc;

/// The identity of the allocation the marks share.
#[must_use]
pub(crate) fn marks_key(marks: &SharedMarks) -> AllocationKey {
    AllocationKey(Arc::as_ptr(&marks.0).addr())
}

/// The capacity of the allocation the marks share.
#[must_use]
pub(crate) fn marks_capacity(marks: &SharedMarks) -> ElementCapacity {
    ElementCapacity(marks.0.capacity())
}
