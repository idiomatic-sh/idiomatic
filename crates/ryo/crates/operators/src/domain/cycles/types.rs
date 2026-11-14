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

/// Engine-prepared adjacency, read through a port: the graph keeps its own layout.
pub trait CycleDriver {
    /// How many nodes the graph has.
    fn nodes(&self) -> NodeCount;
    /// The value of a node, or none for a number the graph does not have.
    fn value(&self, node: NodeNumber) -> Option<Value>;
    /// The node the edge at `place` leaving `node` leads to, or none when `node` has no such edge.
    fn edge(&self, node: NodeNumber, place: EdgePlace) -> Option<NodeNumber>;
}

/// The work arrays of one search, kept by the engine between invocations so they keep their capacity.
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
}

/// An engine driver that lends the prepared adjacency of a cycle search.
pub trait CyclesEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's prepared adjacency of a cycle search.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no adjacency is bound.
    fn cycles(
        &mut self,
    ) -> Result<&dyn CycleDriver, crate::domain::contract::types::PortableOperatorFault>;
}

/// The graph a cycle search reads: an edge row's source column and destination column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CycleParts {
    /// The source column of an edge row.
    pub(crate) source: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The destination column of an edge row.
    pub(crate) destination: ryo_values::domain::plan_parts::types::ColumnIndex,
}
