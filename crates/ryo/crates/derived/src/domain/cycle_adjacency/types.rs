//! The types of the `cycles` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::Value;

/// The number of one node of a cycle graph, in first-seen order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeSlot(pub usize);

/// Where the edges leaving one node begin in the leaving list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LeavingStart(pub usize);

/// Dense adjacency in first-seen node order, with duplicate edges kept.
#[derive(Debug, Default)]
pub struct CycleGraph {
    /// The node values in first-seen order.
    pub values: Vec<Value>,
    /// Where each node's edges begin, with one entry more than there are nodes.
    pub starts: Vec<LeavingStart>,
    /// The destination of each edge, grouped by source.
    pub leaving: Vec<NodeSlot>,
}
