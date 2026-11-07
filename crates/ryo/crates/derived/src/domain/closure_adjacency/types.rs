//! The types of the `closure` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::allocation::types::ElementCapacity;
use crate::domain::equality::types::KeyIndex;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::Table;

/// The closure adjacency of one partition: its edges, their index and their distinct sources.
#[derive(Debug, Default)]
pub struct ClosurePartition {
    /// The partition's key tuple.
    pub key: Vec<Value>,
    /// The source and destination of each edge.
    pub table: Table,
    /// The index of the edges by source.
    pub index: KeyIndex,
    /// The distinct sources in order.
    pub sources: Vec<Value>,
}

/// The capacity of the buffers of a closure partition that are not a table or an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureCapacities {
    /// The capacity of the key tuple.
    pub key: ElementCapacity,
    /// The capacity of the sources.
    pub sources: ElementCapacity,
}
