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
    /// Executable registered operation with normalized local input numbers.
    pub operation: ryo_values::domain::plans::types::Operation,
    /// Ordered typed dependencies retained for the central executor.
    pub inputs: Vec<NodeInput>,
    /// Metadata-only output kinds, validated before graph admission.
    pub kinds: Vec<ryo_language::domain::relations::types::CellKind>,
    /// Invalid synthetic plans keep ordinary error order instead of DAG admission.
    pub schema_valid: bool,
    /// Transitive physical facts required before the application admits this node.
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
    /// Graph outputs consumed by residual operations, in original step order.
    pub boundaries: Vec<Vec<(usize, NodeId)>>,
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

/// The domain executor asks its application owner for an already scheduled DAG
/// output. Implementations own generation state; primitives never schedule it.
pub(crate) trait DagExecution {
    fn owned(&self, rule: usize, step: usize) -> Option<NodeId>;
    fn demand<'f>(
        &self,
        node: NodeId,
        read: &dyn Fn(
            Relation,
        ) -> Result<
            crate::domain::table_model::types::Rows<'f>,
            crate::domain::operating::errors::OperatingError,
        >,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
    ) -> Result<Arc<Table>, crate::domain::operating::errors::OperatingError>;
}
