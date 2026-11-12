//! The types of the `footprint` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_derived::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};
use ryo_derived::domain::vertices::types::{PartitionId, VertexId, VertexKey};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// Which kind of allocation a shared backing is.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::accounting::footprint::types::BackingKind;
/// assert_ne!(BackingKind(9), BackingKind(11));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BackingKind(pub u8);

/// Physical immutable sub-allocation description, without any lease/admission class.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::accounting::footprint::types::{BackingKind, SharedBacking};
/// use ryo_derived::domain::allocation::types::{AllocationBytes, AllocationKey, ElementCapacity};
/// let backing = SharedBacking {
///     tag: BackingKind(11),
///     allocation_key: AllocationKey(1),
///     bytes: AllocationBytes(64),
///     position_capacity: ElementCapacity(0),
/// };
/// assert_eq!(backing.bytes, AllocationBytes(64));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedBacking {
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
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::accounting::footprint::types::PayloadFootprint;
/// use ryo_derived::domain::allocation::types::AllocationBytes;
/// let footprint = PayloadFootprint { owned_bytes: AllocationBytes(0), shared: Vec::new() };
/// assert!(footprint.shared.is_empty());
/// ```
#[derive(Debug)]
pub struct PayloadFootprint {
    /// The bytes only this payload holds.
    pub owned_bytes: AllocationBytes,
    /// The shared allocations it holds.
    pub shared: Vec<SharedBacking>,
}

/// One entry of the partition table of a compact graph, as the hash table stores it.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::accounting::footprint::types::PartitionEntry;
/// use ryo_derived::domain::vertices::types::PartitionId;
/// let entry = PartitionEntry(std::sync::Arc::from([]), PartitionId(0));
/// assert_eq!(entry.1, PartitionId(0));
/// ```
#[derive(Debug)]
pub struct PartitionEntry(pub Arc<[Value]>, pub PartitionId);

/// One entry of the vertex table of a compact graph, as the hash table stores it.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::accounting::footprint::types::VertexEntry;
/// let _ = std::mem::size_of::<VertexEntry>();
/// ```
#[derive(Clone, Copy, Debug)]
pub struct VertexEntry(pub VertexKey, pub VertexId);
