//! The types of the `physical_plan` concept: the plain data that crosses the operators' seam and the two traits the engine
//! implements for a run. The plan's and the buffers' own fields stay private to operators.

pub mod file_buffers;
pub mod logical_program;
pub mod lowering;
pub mod physical_plan;

use crate::domain::contract::types::{PortableOperatorFault, RenderKind};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{ColumnsView, NodeId, Relation, TextHash, Value};
use crate::domain::select::types::SelectParts;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::{ObservationBindingId, ParameterId, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;

/// A node of a program, by its place in the program: earlier nodes have lower ids. Not a tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanNodeId(pub u32);

/// A rule of a program, by its place in rule order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleOrdinal(pub u32);

/// What one input of a node reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanInput {
    /// An earlier node's rows.
    Node(PlanNodeId),
    /// A relation's rows for the open file, lent by the facts.
    Relation(Relation),
    /// The rows a table parameter is bound to.
    Bound(ParameterId),
    /// The rows an observation binding answers.
    Observation(ObservationBindingId),
}

/// A rule's emitted node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Emitted {
    /// The rule.
    pub(crate) rule: RuleOrdinal,
    /// The node it emits.
    pub(crate) node: PlanNodeId,
}

/// The inputs of a node, in the order its kind reads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlanInputList<'p>(pub &'p [PlanInput]);

/// The parameter bindings a node's operation reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeBindings<'p>(pub &'p [ParameterBinding]);

/// One node of a logical program: the operation it is, borrowed from its rule's plan, what it reads, the bindings it reads
/// and the scope it runs in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalNode<'p> {
    /// The operation, borrowed: never cloned.
    pub operation: &'p Operation,
    /// What its inputs read.
    pub inputs: PlanInputList<'p>,
    /// The bindings it reads.
    pub bindings: NodeBindings<'p>,
    /// Whether it runs on one file's rows or after the files.
    pub scope: StepScope,
}

/// A node's rows in a file's buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRows<'b> {
    /// The node was run and its rows are held.
    Materialized(&'b Table),
    /// The node was not run in this file, or its rows were released.
    Absent,
}

/// A relation's rows as the facts lend them for the open file.
#[derive(Clone, Copy, Debug)]
pub enum FactRows<'a> {
    /// The rows, borrowed in place: read as they are, never decoded into a table.
    Lent(ColumnsView<'a>),
    /// The facts refused the relation: a fault, never empty rows.
    Refused,
}

/// The tree nodes a root may start from, in tree order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidates<'a>(pub &'a [NodeId]);

/// The bytes of the open file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenFileBytes<'a>(pub &'a [u8]);

/// The nodes a batch of renderings asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderRequests<'a>(pub &'a [NodeId]);

/// What the provider answered for each rendering of a batch, in request order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderOutcomes(pub Vec<RenderKind>);

/// The rows a run reads besides its own nodes', which the engine holds for the open file.
pub trait PlanInputs {
    /// A relation's rows, lent in place.
    fn facts(&self, relation: Relation) -> FactRows<'_>;
    /// The tree nodes a root node starts from.
    fn roots(&self, root: PlanNodeId) -> Candidates<'_>;
    /// The rows a table parameter is bound to.
    fn bound(&self, parameter: ParameterId) -> &Table;
    /// The rows an observation binding answers.
    fn observation(&self, input: ObservationBindingId) -> &Table;
}

/// What a kernel borrows from the engine besides rows: the complete inventory.
pub trait Resources {
    /// Where the texts of the open file's cells are: its symbols, its bytes, the paths of the run's files and the workers'
    /// texts, which the moved comparisons decide over unchanged.
    fn texts(&self) -> Texts<'_>;
    /// The cell of a text, interned in the open file's symbols.
    fn intern(&mut self, text: CellText<'_>) -> Value;
    /// A text interned as a derived symbol of the open file.
    fn mint(&mut self, text: CellText<'_>) -> Value;
    /// The bytes of the open file.
    fn source(&self) -> OpenFileBytes<'_>;
    /// The tree of the open file.
    fn tree(&self) -> &Elements;
    /// Renders each requested node in its syntax role.
    ///
    /// # Errors
    ///
    /// The engine's refusal: no semantic session, or a batch of another shape.
    fn render(
        &mut self,
        requests: RenderRequests<'_>,
    ) -> Result<RenderOutcomes, PortableOperatorFault>;
}

/// The stage of the swap that lowers a kind: `F2` is `ServedAt(2)`. Branch only: deleted with the last unlowered kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServedAt(pub u8);

/// A node whose kind has no lowering yet, and the stage that lowers it. Branch only: deleted with the last unlowered kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Unserved {
    /// The node.
    pub node: PlanNodeId,
    /// The stage that lowers its kind.
    pub until: ServedAt,
}

/// What a node of the physical plan reads its rows from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RowSource {
    /// A relation's rows, lent by the facts.
    Facts(Relation),
    /// A tree relation's rows, read from the open file's elements.
    Tree(Relation),
    /// An earlier node's materialized rows.
    Node(PlanNodeId),
}

/// The place of a shared index among a plan's indexes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexPlace(pub(crate) usize);

/// How many node readers a node has, or has left in the open file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NodeReaders(pub(crate) u32);

/// Whether a node's rows outlive its node readers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Retention {
    /// Released when its last reader is done.
    Released,
    /// Kept until the engine releases it: a rule's emit or a held node.
    Retained,
}

/// A `hash_index` shared by every lookup of one relation's column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyedIndex {
    /// The relation it indexes.
    pub(crate) relation: Relation,
    /// The key column.
    pub(crate) column: ColumnIndex,
    /// The lookup nodes that read it, in node order: one build answers all of them.
    pub(crate) lookups: Vec<PlanNodeId>,
}

/// The key a select fixes in its infallible prefix: a name column of a relation equal to a written text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectKey {
    /// The relation.
    pub(crate) relation: Relation,
    /// The name column.
    pub(crate) column: ColumnIndex,
    /// The hash of the written text.
    pub(crate) key: TextHash,
}

/// What a run lends its kernels: the engine's rows and resources, and the matching semantics.
pub(crate) struct KernelRun<'r> {
    /// The rows the engine holds for the open file.
    pub(crate) inputs: &'r dyn PlanInputs,
    /// What the kernels borrow besides rows.
    pub(crate) resources: &'r mut dyn Resources,
    /// The matching semantics the comparisons compile and match through.
    pub(crate) matcher: crate::domain::patterns::types::PatternMatcher<'r>,
}

/// One filter's comparisons over rows of one shape, against the open file's texts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FilterKernel<'f> {
    /// The comparisons, the kept columns and the bindings.
    pub(crate) select: &'f SelectParts,
    /// What each column of a row holds.
    pub(crate) kinds: &'f [ryo_relations::domain::relations::types::CellKind],
    /// The open file's texts.
    pub(crate) texts: Texts<'f>,
    /// How many names the open file has.
    pub(crate) names: crate::domain::comparing::types::NameCount,
}

/// A materialized input, taken out of the buffers while its reader reads it, with the fault it raised after its rows.
#[derive(Debug)]
pub(crate) struct TakenRows {
    /// The rows.
    pub(crate) rows: Table,
    /// The fault raised after them.
    pub(crate) pending: Option<PortableOperatorFault>,
}

/// Rows a kernel reads in place: a relation's columns or a node's table.
#[derive(Clone, Copy, Debug)]
pub(crate) enum InputRows<'a> {
    /// A relation's columns, lent by the facts.
    Facts(&'a ColumnsView<'a>),
    /// A tree relation's rows, read from the elements.
    Tree(&'a TreeRead<'a>),
    /// A table of rows.
    Table(&'a Table),
}

/// Rows keyed by key columns: the rows and their key columns.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyedSide<'a> {
    /// The rows.
    pub(crate) rows: InputRows<'a>,
    /// Their key columns.
    pub(crate) on: &'a [ColumnIndex],
}

/// One row of keyed rows.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyedRowAt<'a> {
    /// The rows and their key columns.
    pub(crate) side: KeyedSide<'a>,
    /// The row.
    pub(crate) row: ryo_values::domain::rows::types::RowIndex,
}

/// The place in a table's cells where the row being written starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CellStart(pub(crate) usize);

/// A tree relation's rows as one node reads them: the open file's elements at the tree nodes the engine named, in tree order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeRead<'a> {
    /// Which rows of the tree these are.
    pub(crate) form: crate::domain::tree_cells::types::TreeForm,
    /// What each column holds.
    pub(crate) kinds: &'static [ryo_relations::domain::relations::types::CellKind],
    /// The open file's elements.
    pub(crate) elements: &'a Elements,
    /// The tree nodes whose rows these are, in tree order.
    pub(crate) candidates: &'a [NodeId],
}
