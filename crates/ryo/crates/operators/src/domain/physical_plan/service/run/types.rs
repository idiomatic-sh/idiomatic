//! The runner's own data: the rows a node's kernels read, taken from the buffers while they read them.
use crate::domain::physical_plan::types::{InputRows, PlanNodeId, TakenRows, TreeRead};
use ryo_relations::domain::relations::types::ColumnsView;

/// The rows a node's kernels read: its first input's, and its second's when it reads two.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KernelInputs<'a> {
    /// The first input's rows.
    pub(crate) input: InputRows<'a>,
    /// The second input's rows, when the node reads two.
    pub(crate) other: Option<InputRows<'a>>,
    /// The third input's rows, when the node reads three: an axis's domain.
    pub(crate) third: Option<InputRows<'a>>,
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
    /// Rows the engine holds for the run, read in place.
    Held(&'a ryo_values::domain::rows::types::Table),
}

/// What a stage's preparation left, before it reads any row: the fault it raised there (an extension's resolution), which
/// stops it before its first row. Its output kinds, as far as its preparation set them, are on the table it writes.
#[derive(Debug, Default)]
pub(crate) struct StagePrepared {
    /// The fault it raised before reading any row.
    pub(crate) fault: Option<crate::domain::contract::types::PortableOperatorFault>,
}

/// The rows a chain of streamed stages reads besides its stages' own: its head stage's source and each probe's right side.
/// A tree relation's rows are read from the elements row by row, so a stage that mints between them holds no borrow of them.
#[derive(Debug)]
pub(crate) enum ChainRows<'r> {
    /// A relation's columns, lent in place.
    Facts(ColumnsView<'r>),
    /// A tree relation's nodes; their elements are lent row by row.
    Tree(crate::domain::physical_plan::types::TreeRoots<'r>),
    /// A tree relation's rows copied out, for a head stage that mints while it reads.
    Copied(ryo_values::domain::rows::types::Table),
    /// A held node and its rows and pending fault, taken while the chain reads them.
    Node(PlanNodeId, TakenRows),
    /// Rows the engine holds for the run.
    Held(&'r ryo_values::domain::rows::types::Table),
    /// The head's rows: a probe's right side that is the head stage's source.
    Head,
    /// An earlier probe's right rows, by their place.
    Right(usize),
}
