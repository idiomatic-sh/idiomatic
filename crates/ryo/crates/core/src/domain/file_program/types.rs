//! Immutable request graph and bounded, invocation-local shared results.

use crate::domain::table_model::types::Table;
use ryo_language::domain::relations::types::Relation;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NodeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeInput {
    Relation(Relation),
    Node(NodeId),
}

#[derive(Debug)]
pub(crate) struct Node {
    pub contract: Arc<crate::domain::operator::contract::types::OperatorContract>,
    pub physical:
        crate::domain::table_model::collections::representations::publication::PreparedPhysical,
    /// Executable registered operation with normalized local input numbers.
    pub operation: ryo_values::domain::plans::types::Operation,
    /// Ordered typed dependencies retained for the central executor.
    pub inputs: Vec<NodeInput>,
    /// Invalid synthetic plans keep ordinary error order instead of DAG admission.
    pub schema_valid: bool,
    /// Transitive physical facts required before the application admits this node.
    pub relations: Vec<Relation>,
}

#[derive(Debug)]
pub(crate) struct RequestGraph {
    /// Producers of declared derived inputs, computed once for the whole request.
    pub derived_sources: Vec<bool>,
    pub plans: Vec<usize>,
    pub nodes: Vec<Node>,
    /// Only internal kept cuts may own a file-local result. Other entries are None.
    pub cuts: Vec<Vec<Option<NodeId>>>,
    /// Unique eligible nodes consumed by each rule, prepared once per request.
    pub consumers: Vec<Vec<NodeId>>,
    /// Graph outputs consumed by residual operations, in original step order.
    pub boundaries: Vec<Vec<(usize, NodeId)>>,
    /// Parent-before-child rule-local live-edge counts, compiled once per request.
    pub rule_liveness: Vec<Vec<(NodeId, usize)>>,
}

/// One physical file generation; no table escapes this value or enters a global cache.
#[derive(Debug)]
pub(crate) struct FileProgram {
    pub graph: Arc<RequestGraph>,
    pub remaining: Vec<usize>,
    pub entries: Vec<(NodeId, Arc<Table>)>,
    pub shared_executions: u64,
    pub peak_retained_cells: usize,
}
