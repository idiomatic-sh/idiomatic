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

/// What a stage's preparation left, before it reads any row: its output kinds, as far as its preparation set them, and the
/// fault it raised there (an extension's resolution, a probe's right side's), which stops it before its first row and
/// leaves its kinds empty.
#[derive(Debug, Default)]
pub(crate) struct StagePrepared {
    /// The kinds of the rows it writes.
    pub(crate) kinds: Vec<ryo_relations::domain::relations::types::CellKind>,
    /// The fault it raised before reading any row.
    pub(crate) fault: Option<crate::domain::contract::types::PortableOperatorFault>,
}

/// The rows a chain of streamed stages reads besides its stages' own: its head stage's source and each probe's right side.
/// A tree relation's rows are read from the elements a chunk at a time, so a stage that mints between chunks holds no borrow
/// of them.
#[derive(Debug)]
pub(crate) enum ChainRows<'r> {
    /// A relation's columns, lent in place.
    Facts(ColumnsView<'r>),
    /// A tree relation's nodes; their elements are lent row by row.
    Tree(crate::domain::physical_plan::types::TreeRoots<'r>),
    /// A held node and its rows and pending fault, taken while the chain reads them.
    Node(PlanNodeId, TakenRows),
    /// Rows the engine holds for the run.
    Held(&'r ryo_values::domain::rows::types::Table),
    /// The head's rows: a probe's right side that is the head stage's source.
    Head,
    /// An earlier probe's right rows.
    Right(crate::domain::physical_plan::types::stream_scratch::value::RightPlace),
    /// A held node a probe reads, not taken yet: taken once every source of the chain is readied.
    Untaken(PlanNodeId),
}

/// One stage of a chain: its node and what it is lowered to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChainStage<'p> {
    /// The stage's node.
    pub(crate) node: PlanNodeId,
    /// Its kernels.
    pub(crate) lowering: &'p crate::domain::physical_plan::types::lowering::value::Lowering,
}

/// A hard fault of a chain and the node of the stage that raised it. A node reads only earlier nodes, so of two faults the
/// earlier node's is the one the held plan, which makes each node before its readers, raises first.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ChainFault {
    /// The stage's node.
    pub(crate) node: PlanNodeId,
    /// The fault.
    pub(crate) fault: crate::domain::contract::types::PortableOperatorFault,
}

/// How the runner readies a source a chain reads: it makes a held node that holds no rows yet. The chain is given it by
/// the runner that calls it, so it never names the runner.
pub(crate) type Readied = fn(
    &crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
    &mut crate::domain::physical_plan::types::KernelRun<'_>,
    crate::domain::physical_plan::types::RowSource,
    &mut crate::domain::physical_plan::types::file_buffers::value::FileBuffers,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault>;

/// What the stages of a chain write through while its rows are pushed: the plan, the run, the worker's buffers, the rows
/// the chain reads besides its stages' own, the tail's table, and the chain's first hard fault.
pub(crate) struct Push<'p, 'k, 'r> {
    /// The plan the chain's nodes are of.
    pub(crate) plan: &'p crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
    /// The engine's rows and resources, and the matching semantics.
    pub(crate) run: &'k mut crate::domain::physical_plan::types::KernelRun<'r>,
    /// The worker's buffers: each node's decisions, the text scratch and the shared indexes.
    pub(crate) buffers:
        &'k mut crate::domain::physical_plan::types::file_buffers::value::FileBuffers,
    /// The head stage's source.
    pub(crate) head: &'k ChainRows<'r>,
    /// Each probe's right side, in chain order.
    pub(crate) rights: &'k [ChainRows<'r>],
    /// The tail's rows.
    pub(crate) tail: &'k mut ryo_values::domain::rows::types::Table,
    /// The chain's first hard fault in the held plan's order.
    pub(crate) hard: Option<ChainFault>,
    /// The tail stage's place: a push handed every stage from the head's to it is the head stage's.
    pub(crate) last: crate::domain::physical_plan::types::stream_scratch::value::StagePlace,
    /// The chain's first probe's place, if it has one.
    pub(crate) first_probe:
        Option<crate::domain::physical_plan::types::stream_scratch::value::StagePlace>,
    /// Spike (ryo-22c7): the head probe's lowering when it reads restricted right rows through an index of its own.
    pub(crate) bound: Option<(
        PlanNodeId,
        &'p crate::domain::physical_plan::types::lowering::value::Lowering,
    )>,
}

/// Whether a chain's head stage reads on: not once a stage before the tail stopped with no probe at or before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeadReads {
    /// It reads its next rows.
    On,
    /// It reads no more.
    Ended,
}

/// What one stage's preparation reads: the plan, the chain's rows besides its stages', the chain's nodes and the stage, and
/// the tail's table.
pub(crate) struct Prepare<'p, 'k, 'r> {
    /// The plan the chain's nodes are of.
    pub(crate) plan: &'p crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
    /// The head stage's source.
    pub(crate) head: &'k ChainRows<'r>,
    /// Each probe's right side read so far, in chain order.
    pub(crate) rights: &'k [ChainRows<'r>],
    /// The chain's nodes, from the head stage to the tail.
    pub(crate) nodes: &'k [PlanNodeId],
    /// The stage.
    pub(crate) stage: ChainStage<'p>,
    /// The tail's table.
    pub(crate) tail: &'k mut ryo_values::domain::rows::types::Table,
}
