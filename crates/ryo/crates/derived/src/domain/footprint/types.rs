//! The types of the `footprint` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};
use crate::domain::vertices::types::{PartitionId, VertexKey, VertexId};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// Which kind of allocation a shared backing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackingTag(pub u8);

/// Physical immutable sub-allocation description, without any lease/admission class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedBacking {
    /// Which kind of allocation it is.
    pub tag: BackingTag,
    /// The identity of the allocation.
    pub allocation_key: AllocationKey,
    /// The bytes it occupies.
    pub bytes: AllocationBytes,
    /// The capacity of its positions, zero when it holds none.
    pub position_capacity: ElementCapacity,
}

/// What one payload occupies: its own bytes and each shared backing.
#[derive(Debug)]
pub struct PayloadFootprint {
    /// The bytes only this payload holds.
    pub owned_bytes: AllocationBytes,
    /// The shared allocations it holds.
    pub shared: Vec<SharedBacking>,
}

/// One entry of the partition table of a compact graph, as the hash table stores it.
pub struct PartitionEntry(pub Arc<[Value]>, pub PartitionId);

/// One entry of the vertex table of a compact graph, as the hash table stores it.
pub struct VertexEntry(pub VertexKey, pub VertexId);
