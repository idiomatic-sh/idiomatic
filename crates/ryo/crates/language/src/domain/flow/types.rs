//! The types of the `flow` concept: a file's control flow graphs as its language's read gave them, by block number and element
//! position.

use ryo_relations::domain::relations::types::{Flag, Position};

/// A basic block, by its number among the file's blocks, from zero, in the order the read gave them.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::BlockNumber;
///
/// assert!(BlockNumber(0) < BlockNumber(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockNumber(pub u32);

/// A step's place among all of its block's steps, from zero, steps with no element (an implicit return, an unreachable mark)
/// counted, so the places are oxc's.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::StepOrdinal;
///
/// assert!(StepOrdinal(0) < StepOrdinal(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepOrdinal(pub u32);

/// An edge's place among the edges leaving its source block, in the order the language's flow analysis follows them, from 0.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::EdgeOrdinal;
///
/// assert!(EdgeOrdinal(0) < EdgeOrdinal(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeOrdinal(pub u32);

/// How control passes along an edge: the closed vocabulary of `flow-edge.edge-kind`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::service::flow_edge_word;
/// use ryo_language::domain::flow::types::FlowEdgeKind;
///
/// assert_eq!(flow_edge_word(FlowEdgeKind::Backedge).0, "backedge");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowEdgeKind {
    /// Falling through to what follows, or a test's false branch.
    Normal,
    /// A test's true branch, or a jump to a label.
    Jump,
    /// From a loop's end back to its start.
    Backedge,
    /// Into the block that joins the branches of a `finally`.
    Join,
    /// Into a `finally` block.
    Finalize,
    /// From the block a function is written in to the function's first block.
    NewFunction,
    /// Into the block after a return, throw, break or continue.
    Unreachable,
    /// Into a `catch`, from a block that can throw.
    ErrorExplicit,
    /// Out of the function, from a block that can throw outside any `try`.
    ErrorImplicit,
}

/// What a step with an element does: the closed vocabulary of `flow-step.step-kind`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::service::flow_step_word;
/// use ryo_language::domain::flow::types::FlowStepKind;
///
/// assert_eq!(flow_step_word(FlowStepKind::ReturnUndefined).0, "return-undefined");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FlowStepKind {
    /// A statement.
    Statement,
    /// A return with a value.
    Return,
    /// A return with no value, which returns `undefined`.
    ReturnUndefined,
    /// A break with no label.
    Break,
    /// A break to a label.
    BreakLabeled,
    /// A continue with no label.
    Continue,
    /// A continue to a label.
    ContinueLabeled,
    /// A throw.
    Throw,
    /// A test that chooses the branch.
    Condition,
    /// The step of a `for … of` loop.
    IterationOf,
    /// The step of a `for … in` loop.
    IterationIn,
}

/// One basic block of a file's control flow graphs.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::FlowBlockFact;
/// use ryo_relations::domain::relations::types::Flag;
///
/// let after_a_return = FlowBlockFact { reachable: Flag::Lowered, live: Flag::Lowered, implicit_return: Flag::Lowered };
/// assert_eq!(after_a_return.reachable, Flag::Lowered);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowBlockFact {
    /// Whether the graph's builder marks it reachable.
    pub reachable: Flag,
    /// Whether control reaches it once a loop whose test is the literal `true` exits only through a break.
    pub live: Flag,
    /// Whether it ends its function by falling off its end.
    pub implicit_return: Flag,
}

/// One edge between two blocks.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::{BlockNumber, EdgeOrdinal, FlowEdgeFact, FlowEdgeKind};
///
/// let back = FlowEdgeFact { source: BlockNumber(2), target: BlockNumber(1), kind: FlowEdgeKind::Backedge, ordinal: EdgeOrdinal(0) };
/// assert!(back.target < back.source);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowEdgeFact {
    /// The block it leaves.
    pub source: BlockNumber,
    /// The block it enters.
    pub target: BlockNumber,
    /// How control passes.
    pub kind: FlowEdgeKind,
    /// Its place among the edges leaving `source`, in the order the flow analysis follows them.
    pub ordinal: EdgeOrdinal,
}

/// One step of a block that is an element of the file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::{BlockNumber, FlowStepFact, FlowStepKind, StepOrdinal};
/// use ryo_relations::domain::relations::types::Position;
///
/// let step = FlowStepFact { block: BlockNumber(0), ordinal: StepOrdinal(1), kind: FlowStepKind::Statement, node: Position(4) };
/// assert_eq!(step.ordinal, StepOrdinal(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowStepFact {
    /// The block it is in.
    pub block: BlockNumber,
    /// Its place in the block.
    pub ordinal: StepOrdinal,
    /// What it does.
    pub kind: FlowStepKind,
    /// The element it is.
    pub node: Position,
}

/// The block an element is evaluated in.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::{BlockNumber, FlowMemberFact};
/// use ryo_relations::domain::relations::types::Position;
///
/// let member = FlowMemberFact { node: Position(3), block: BlockNumber(1) };
/// assert_eq!(member.block, BlockNumber(1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowMemberFact {
    /// The element.
    pub node: Position,
    /// Its block.
    pub block: BlockNumber,
}

/// A file's control flow graphs, kept with its table: empty for a file whose read gave none, and emptied with the table, keeping
/// its room for the next file.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::flow::types::FlowGraph;
///
/// let graph = FlowGraph::default();
/// assert!(graph.blocks.is_empty() && graph.edges.is_empty());
/// ```
#[derive(Debug, Default)]
pub struct FlowGraph {
    /// The blocks, by number.
    pub blocks: Vec<FlowBlockFact>,
    /// The edges, in the order the read gave them.
    pub edges: Vec<FlowEdgeFact>,
    /// The steps with an element, by block, then place.
    pub steps: Vec<FlowStepFact>,
    /// The elements' blocks, in the order the read gave them.
    pub members: Vec<FlowMemberFact>,
}
