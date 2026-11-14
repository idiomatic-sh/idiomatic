//! The types of the `file_pass` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::NodeId;
use ryo_values::domain::plan_parts::types::LiteralText;

/// The tree nodes each node of a planned program starts from in the open file, made once per file into buffers kept across
/// files.
#[derive(Debug, Default)]
pub(crate) struct FileRoots {
    /// Each root kind of the program with the nodes that start from it.
    pub(crate) kinds: Vec<RootKindNodes>,
    /// Where each node starts from, by node.
    pub(crate) starts: Vec<TreeStart>,
    /// The open file's tree nodes of each rooted node's kinds, in tree order, by node.
    pub(crate) lists: Vec<Vec<NodeId>>,
    /// Every tree node of the open file, in tree order.
    pub(crate) all: Vec<NodeId>,
}

/// A root kind of a program, held across files: its text and the nodes that start from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootKindNodes {
    /// The kind's text, as the plans write it.
    pub(crate) kind: LiteralText,
    /// The nodes that start from it.
    pub(crate) nodes: Vec<PlanNodeId>,
}

/// Whether a node starts from the tree nodes of its root kinds or from every tree node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TreeStart {
    /// It starts from its root kinds' tree nodes.
    FromKinds,
    /// It starts from every tree node.
    #[default]
    FromEveryNode,
}
