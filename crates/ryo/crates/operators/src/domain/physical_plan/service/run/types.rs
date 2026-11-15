//! The runner's own types: what one node's kernels read.
use crate::domain::physical_plan::types::{InputRows, PlanNodeId, TakenRows, TreeRead};
use ryo_relations::domain::relations::types::ColumnsView;

/// The rows a node's kernels read: its first input's, and its second's when it reads two.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KernelInputs<'a> {
    /// The first input's rows.
    pub(crate) input: InputRows<'a>,
    /// The second input's rows, when the node reads two.
    pub(crate) other: Option<InputRows<'a>>,
}

/// The rows a node's kernel reads: a relation's, lent in place, or an earlier node's, taken while it reads them.
#[derive(Debug)]
pub(crate) enum Reading<'a> {
    /// A relation's columns.
    Facts(ColumnsView<'a>),
    /// A tree relation's rows.
    Tree(TreeRead<'a>),
    /// A tree relation's rows copied out, for a node that holds the resources mutably while it reads them.
    Copied(ryo_values::domain::rows::types::Table),
    /// An earlier node and its rows and pending fault.
    Node(PlanNodeId, TakenRows),
}
