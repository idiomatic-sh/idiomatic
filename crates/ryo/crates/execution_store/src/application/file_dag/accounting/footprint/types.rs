//! The types of the `footprint` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_derived::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};

/// Which kind of allocation a shared backing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BackingKind(pub u8);

/// Physical immutable sub-allocation description, without any lease/admission class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SharedBacking {
    /// Which kind of allocation it is.
    pub tag: BackingKind,
    /// The identity of the allocation.
    pub allocation_key: AllocationKey,
    /// The bytes it occupies.
    pub bytes: AllocationBytes,
    /// The capacity of its positions, zero when it holds none.
    pub position_capacity: ElementCapacity,
}

/// What one payload occupies: its own bytes and each shared backing.
#[derive(Debug)]
pub(crate) struct PayloadFootprint {
    /// The bytes only this payload holds.
    pub owned_bytes: AllocationBytes,
    /// The shared allocations it holds.
    pub shared: Vec<SharedBacking>,
}
