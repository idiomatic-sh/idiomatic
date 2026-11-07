//! The types of the `vertices` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::Value;

/// The number of one partition of a graph, in first-seen order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PartitionId(pub u32);

/// The number of one vertex of a graph, in first-seen order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VertexId(pub u32);

/// Where the edges leaving one vertex begin in the neighbor list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeOffset(pub u32);

/// Which end of an edge a list is ordered by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
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
