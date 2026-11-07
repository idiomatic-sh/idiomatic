//! Immutable shared positions: the value, its weak handle and their behaviour, owned together.
use crate::domain::allocation::types::{AllocationKey, ElementCapacity};
use crate::domain::positions::types::RowPosition;
use std::sync::{Arc, Weak};

/// Immutable shared positions; its allocation identity never depends on liveness.
#[derive(Clone, Debug)]
pub struct SharedPositions(pub(crate) Arc<Vec<RowPosition>>);

/// Weak sharing observes an allocation without retaining row IDs or mutable map equality.
#[derive(Clone, Debug)]
pub struct WeakPositions {
    allocation_key: AllocationKey,
    capacity: ElementCapacity,
    rows: Weak<Vec<RowPosition>>,
}

impl WeakPositions {
    /// The identity of the observed allocation.
    #[must_use]
    pub fn allocation_key(&self) -> AllocationKey {
        self.allocation_key
    }
    /// The capacity of the observed allocation when it was observed.
    #[must_use]
    pub fn capacity(&self) -> ElementCapacity {
        self.capacity
    }
    /// The positions, while some owner still holds them.
    #[must_use]
    pub fn upgrade(&self) -> Option<SharedPositions> {
        self.rows.upgrade().map(SharedPositions)
    }
}

impl SharedPositions {
    /// Shares the positions an owner built.
    #[must_use]
    pub fn new(rows: Vec<RowPosition>) -> Self {
        Self(Arc::new(rows))
    }
    /// The positions.
    #[must_use]
    pub fn as_slice(&self) -> &[RowPosition] {
        self.0.as_slice()
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
    /// A weak handle on the same allocation.
    #[must_use]
    pub fn downgrade(&self) -> WeakPositions {
        WeakPositions {
            allocation_key: self.allocation_key(),
            capacity: self.capacity(),
            rows: Arc::downgrade(&self.0),
        }
    }
}
