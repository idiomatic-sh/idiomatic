//! The types of the `vertices` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::Value;

/// The number of one partition of a graph, in first-seen order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PartitionId(pub u32);

/// The number of one vertex of a graph, in first-seen order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(pub u32);

/// Where the edges leaving each vertex begin in the neighbor list: the bare offsets inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OffsetList(pub Vec<u32>);

/// The neighbor of each edge, grouped by vertex: the bare vertex numbers inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NeighborList(pub Vec<u32>);

/// The partition of each vertex: the bare partition numbers inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartitionList(pub Vec<u32>);

/// The parent of each vertex, the storage sentinel for a root: the bare vertex numbers inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParentList(pub Vec<u32>);

/// Borrowed parents: the bare vertex numbers inside are named here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentSlice<'a>(pub &'a [u32]);

/// Which end of an edge a list is ordered by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeDirection {
    /// Edges listed by their source.
    Forward,
    /// Edges listed by their destination.
    Backward,
}

/// A vertex value within its partition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexKey {
    /// The partition the vertex belongs to.
    pub partition: PartitionId,
    /// The vertex value.
    pub value: Value,
}
