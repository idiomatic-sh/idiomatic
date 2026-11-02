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
    /// Transitive physical facts needed for a complete result. Canonical
    /// operation copies are discarded after request compilation.
    pub relations: Vec<Relation>,
}

#[derive(Debug)]
pub(crate) struct RequestGraph {
    pub plans: Vec<usize>,
    pub nodes: Vec<Node>,
    /// Only internal kept cuts may own a file-local result. Other entries are None.
    pub cuts: Vec<Vec<Option<NodeId>>>,
    /// Unique eligible nodes consumed by each rule, prepared once per request.
    pub consumers: Vec<Vec<NodeId>>,
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
