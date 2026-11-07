//! The types of the `sharing` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::adjacency::types::compact_graph::value::CompactGraph;
use crate::domain::allocation::types::{AllocationKey, ElementCapacity};
use crate::domain::equality::types::KeyIndex;
use std::sync::{Arc, Weak};

/// Immutable shared positions; its allocation identity never depends on liveness.
#[derive(Clone, Debug)]
pub struct SharedPositions(pub Arc<Vec<u32>>);

/// A weak hold on shared positions, which retains nothing.
#[derive(Clone, Debug)]
pub struct WeakRows(pub Weak<Vec<u32>>);

/// Weak sharing observes an allocation without retaining row IDs or mutable map equality.
#[derive(Clone, Debug)]
pub struct WeakPositions {
    /// The identity of the observed allocation.
    pub allocation_key: AllocationKey,
    /// The capacity of the observed allocation when it was observed.
    pub capacity: ElementCapacity,
    /// The weak hold on the positions.
    pub rows: WeakRows,
}

/// Immutable shared membership marks over a dense element space.
#[derive(Clone, Debug)]
pub struct SharedMarks(pub Arc<Vec<bool>>);

/// A key index shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedKeyIndex(pub Arc<KeyIndex>);

/// A compact graph shared by every holder of the payload it was built into.
#[derive(Clone, Debug)]
pub struct SharedGraph(pub Arc<CompactGraph>);
