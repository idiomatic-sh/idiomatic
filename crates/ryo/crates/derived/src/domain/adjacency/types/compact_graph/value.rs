//! Compact adjacency: the value and its behaviour, owned together.
use crate::domain::adjacency::types::forest_parents::value::ForestParents;
use crate::domain::vertices::types::{Direction, EdgeOffset, PartitionId, VertexId, VertexKey};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// Adjacency carries semantic data only; exact population/generation identity is held by the engine.
#[derive(Debug)]
pub struct CompactGraph {
    pub(crate) partitions: Vec<Arc<[Value]>>,
    pub(crate) partition_ids: hashbrown::HashMap<Arc<[Value]>, PartitionId>,
    pub(crate) vertices: Vec<Value>,
    pub(crate) vertex_partitions: Vec<PartitionId>,
    pub(crate) vertex_ids: hashbrown::HashMap<VertexKey, VertexId>,
    pub(crate) offsets: [Vec<EdgeOffset>; 2],
    pub(crate) neighbors: [Vec<VertexId>; 2],
    pub(crate) forests: [Option<Vec<VertexId>>; 2],
}

impl CompactGraph {
    fn side(direction: Direction) -> usize {
        match direction {
            Direction::Forward => 0,
            Direction::Backward => 1,
        }
    }
    /// The partition of a key tuple.
    #[must_use]
    pub fn partition(&self, tuple: &[Value]) -> Option<PartitionId> {
        self.partition_ids.get(tuple).copied()
    }
    /// The key tuple of a partition.
    #[must_use]
    pub fn partition_values(&self, id: PartitionId) -> Option<&[Value]> {
        self.partitions.get(id.0 as usize).map(|values| values.as_ref())
    }
    /// The vertex values in first-seen order.
    #[must_use]
    pub fn vertices(&self) -> &[Value] {
        &self.vertices
    }
    /// The partition of each vertex.
    #[must_use]
    pub fn vertex_partitions(&self) -> &[PartitionId] {
        &self.vertex_partitions
    }
    /// The vertex of a value within a partition.
    #[must_use]
    pub fn vertex(&self, partition: PartitionId, value: Value) -> Option<VertexId> {
        self.vertex_ids.get(&VertexKey { partition, value }).copied()
    }
    /// Where each vertex's edges begin, for one direction.
    #[must_use]
    pub fn offsets(&self, direction: Direction) -> &[EdgeOffset] {
        &self.offsets[Self::side(direction)]
    }
    /// The neighbor list, for one direction.
    #[must_use]
    pub fn neighbors(&self, direction: Direction) -> &[VertexId] {
        &self.neighbors[Self::side(direction)]
    }
    /// The validated ancestry of one direction, when each vertex has one parent and no cycle.
    #[must_use]
    pub fn forest(&self, direction: Direction) -> Option<ForestParents<'_>> {
        self.forests[Self::side(direction)]
            .as_deref()
            .map(|parents| ForestParents { parents })
    }
}
