//! The types of the `cycles` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_relations::domain::relations::types::Value;

/// Where the edges leaving each node begin in the leaving list: the bare offsets inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeavingStarts(pub Vec<usize>);

/// The destination node of each edge, grouped by source: the bare node numbers inside are named here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeavingNodes(pub Vec<usize>);

/// Dense adjacency in first-seen node order, with duplicate edges kept.
#[derive(Debug, Default)]
pub struct CycleGraph {
    /// The node values in first-seen order.
    pub values: Vec<Value>,
    /// Where each node's edges begin, with one entry more than there are nodes.
    pub starts: LeavingStarts,
    /// The destination of each edge, grouped by source.
    pub leaving: LeavingNodes,
}
