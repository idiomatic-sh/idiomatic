//! Compact adjacency: the value and its behaviour, owned together.
use crate::domain::adjacency::types::forest_parents::value::ForestParents;
use crate::domain::vertices::types::{
    EdgeDirection, NeighborList, OffsetList, ParentList, ParentSlice, PartitionId, PartitionList,
    VertexId, VertexKey,
};
use ryo_relations::domain::relations::types::Value;
use std::sync::Arc;

/// Adjacency carries semantic data only; exact population/generation identity is held by the engine.
#[derive(Debug)]
pub struct CompactGraph {
    pub(crate) partitions: Vec<Arc<[Value]>>,
    pub(crate) partition_ids: hashbrown::HashMap<Arc<[Value]>, PartitionId>,
    pub(crate) vertices: Vec<Value>,
    pub(crate) vertex_partitions: PartitionList,
    pub(crate) vertex_ids: hashbrown::HashMap<VertexKey, VertexId>,
    pub(crate) offsets: [OffsetList; 2],
    pub(crate) neighbors: [NeighborList; 2],
    pub(crate) forests: [Option<ParentList>; 2],
}

impl CompactGraph {
    /// The partition of a key tuple.
    #[must_use]
    pub fn partition(&self, tuple: &[Value]) -> Option<PartitionId> {
        self.partition_ids.get(tuple).copied()
    }
    /// The key tuple of a partition.
    #[must_use]
    pub fn partition_values(&self, id: PartitionId) -> Option<&[Value]> {
        self.partitions
            .get(usize::from(id))
            .map(|values| values.as_ref())
    }
    /// The vertex values in first-seen order.
    #[must_use]
    pub fn vertices(&self) -> &[Value] {
        &self.vertices
    }
    /// The partition of each vertex.
    #[must_use]
    pub fn vertex_partitions(&self) -> &PartitionList {
        &self.vertex_partitions
    }
    /// The vertex of a value within a partition.
    #[must_use]
    pub fn vertex(&self, partition: PartitionId, value: Value) -> Option<VertexId> {
        self.vertex_ids
            .get(&VertexKey { partition, value })
            .copied()
    }
    /// Where each vertex's edges begin, for one direction.
    #[must_use]
    pub fn offsets(&self, direction: EdgeDirection) -> &OffsetList {
        match direction {
            EdgeDirection::Forward => &self.offsets[0],
            EdgeDirection::Backward => &self.offsets[1],
        }
    }
    /// The neighbor list, for one direction.
    #[must_use]
    pub fn neighbors(&self, direction: EdgeDirection) -> &NeighborList {
        match direction {
            EdgeDirection::Forward => &self.neighbors[0],
            EdgeDirection::Backward => &self.neighbors[1],
        }
    }
    /// The validated ancestry of one direction, when each vertex has one parent and no cycle.
    #[must_use]
    pub fn forest(&self, direction: EdgeDirection) -> Option<ForestParents<'_>> {
        let parents = match direction {
            EdgeDirection::Forward => self.forests[0].as_ref(),
            EdgeDirection::Backward => self.forests[1].as_ref(),
        };
        parents.map(|parents| ForestParents {
            parents: ParentSlice(&parents.0),
        })
    }
}
