//! Borrowed adjacency port and caller-owned scratch of one cycle search; no engine authority.
use ryo_relations::domain::relations::types::Value;

/// A node of the graph, by its place among the nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeNumber(pub usize);

/// How many nodes a graph has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeCount(pub usize);

/// An edge, by its place among the edges leaving a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgePlace(pub usize);

/// Where a node's edges start among the edges of every node, laid out node after node.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgeStart(pub(crate) usize);

/// The order a node was first met in, or `UNMET` before it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MeetOrder(pub usize);

/// Whether a flag of the search is raised for a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CycleMark {
    /// Not raised.
    #[default]
    Clear,
    /// Raised.
    Raised,
}

/// An entity on the walk of a search for cycles, with the next edge to try.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WalkStep {
    /// The node.
    pub node: NodeNumber,
    /// The next edge to try.
    pub edge: EdgePlace,
}

/// The work arrays of one search, kept by the engine between invocations so they keep their capacity. The `components`
/// kernel's adjacency is the operators' own:
///
/// ```compile_fail,E0616
/// let scratch = ryo_operators::domain::cycles::types::CycleScratch::default();
/// let _starts = scratch.starts.len();
/// ```
#[derive(Debug, Default)]
pub struct CycleScratch {
    /// The order each node was met in.
    pub order: Vec<MeetOrder>,
    /// The lowest order reachable from each node.
    pub low: Vec<MeetOrder>,
    /// Whether each node is on the open stack.
    pub open: Vec<CycleMark>,
    /// Whether each node is on a cycle.
    pub found: Vec<CycleMark>,
    /// The open nodes.
    pub stack: Vec<NodeNumber>,
    /// The members of the component being closed.
    pub component: Vec<NodeNumber>,
    /// The depth-first walk.
    pub walk: Vec<WalkStep>,
    /// The values of the nodes on a cycle, ascending: the answer.
    pub ordered: Vec<Value>,
    /// Where each node's edges start in `leaving`, by node number, and their end after the last node: the `components`
    /// kernel's adjacency.
    pub(crate) starts: Vec<EdgeStart>,
    /// The node each edge leads to, the edges of a node together, in edge-row order.
    pub(crate) leaving: Vec<NodeNumber>,
}

/// The graph a cycle search reads: an edge row's source column and destination column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CycleParts {
    /// The source column of an edge row.
    pub(crate) source: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ryo_values::domain::plan_parts::types::ColumnIndex,
}

impl crate::domain::kept_room::types::KeptRoom for CycleScratch {
    fn kept(&mut self) {
        self.order.kept();
        self.low.kept();
        self.open.kept();
        self.found.kept();
        self.stack.kept();
        self.component.kept();
        self.walk.kept();
        self.ordered.kept();
        self.starts.kept();
        self.leaving.kept();
    }

}
