//! Immutable shared marks: the value and its behaviour, owned together.
use crate::domain::allocation::types::{AllocationKey, ElementCapacity};
use crate::domain::positions::types::MemberMark;
use std::sync::Arc;

/// Immutable shared membership marks over a dense element space.
#[derive(Clone, Debug)]
pub struct SharedMarks(pub(crate) Arc<Vec<MemberMark>>);

impl SharedMarks {
    /// Shares the marks an owner built.
    #[must_use]
    pub fn new(marks: Vec<MemberMark>) -> Self {
        Self(Arc::new(marks))
    }
    /// The marks.
    #[must_use]
    pub fn as_slice(&self) -> &[MemberMark] {
        &self.0
    }
    /// The identity of the allocation.
    #[must_use]
    pub fn allocation_key(&self) -> AllocationKey {
        AllocationKey(Arc::as_ptr(&self.0) as usize)
    }
    /// The capacity of the allocation.
    #[must_use]
    pub fn capacity(&self) -> ElementCapacity {
        ElementCapacity(self.0.capacity())
    }
}
