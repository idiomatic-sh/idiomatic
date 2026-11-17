//! The types of the `physical_plan` concept: the plain data that crosses the operators' seam and the two traits the engine
//! implements for a run. The plan's and the buffers' own fields stay private to operators.

pub mod bound_observation_place;
pub mod candidates;
pub(crate) mod consumers_done;
pub mod fact_rows;
pub mod file_buffers;
pub mod frontier_place;
pub mod held_rows;
pub(crate) mod keyed_rows;
pub mod logical_program;
pub(crate) mod lowering;
pub mod physical_plan;
pub mod rule_ordinal;
pub(crate) mod shared_index;
pub(crate) mod stream_scratch;
pub(crate) mod table_pool;
pub(crate) mod visited_set;
pub(crate) mod touched_nodes;
pub(crate) mod walk_scratch;

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::table_texts::types::Texts;
use bound_observation_place::value::BoundObservationPlace;
use candidates::value::Candidates;
use fact_rows::value::FactRows;
use frontier_place::value::FrontierPlace;
use held_rows::value::HeldRows;
use rule_ordinal::value::RuleOrdinal;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::{ColumnsView, NodeId, Relation, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::{HostedNumber, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::Table;

/// A node of a program, by its place in the program: earlier nodes have lower ids. Not a tree node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanNodeId(pub u32);

/// The rows a node's kernels read from each of its inputs when it was made in the open file, in its inputs' order: a
/// relation's rows, the tree rows read (the rows a tree probe found), an earlier node's rows; none past its inputs.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::NodeFed;
/// use ryo_values::domain::rows::types::RowCount;
/// let probe = NodeFed([RowCount(2), RowCount(3), RowCount(0)]);
/// assert_eq!(probe.0.iter().map(|rows| rows.0).sum::<usize>(), 5);
/// assert_eq!(NodeFed::default(), NodeFed([RowCount(0); 3]));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeFed(pub [ryo_values::domain::rows::types::RowCount; 3]);

/// What one input of a node reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlanInput {
    /// An earlier node's rows.
    Node(PlanNodeId),
    /// A relation's rows for the open file, lent by the facts.
    Relation(Relation),
    /// What the files handed over for an entry of a rule's frontier.
    Frontier(FrontierPlace),
    /// The rows a bound observation of a rule answers.
    Observation(BoundObservationPlace),
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

/// The bytes of the open file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenFileBytes<'a>(pub &'a [u8]);

/// The nodes a batch of renderings asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderRequests<'a>(pub &'a [ryo_language::domain::semantics::types::RenderRequest]);

/// What the provider answered for each rendering of a batch, in request order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderOutcomes(pub Vec<ryo_language::domain::semantics::types::RenderOutcome>);

/// The rows a run reads besides its own nodes', which the engine holds for the open file.
pub trait PlanInputs {
    /// A relation's rows, lent in place.
    fn facts(&self, relation: Relation) -> FactRows<'_>;
    /// The tree nodes a root node starts from.
    fn roots(&self, root: PlanNodeId) -> Candidates<'_>;
    /// What the files handed over for an entry of a rule's frontier, or the token of why it is withheld.
    fn frontier(&self, place: FrontierPlace) -> HeldRows<'_>;
    /// The rows a bound observation of a rule answers, or the token of why they are withheld.
    fn observation(&self, place: BoundObservationPlace) -> HeldRows<'_>;
    /// The host's rows of a declared relation, or the token of why they are withheld.
    fn hosted(&self, relation: HostedNumber) -> HeldRows<'_>;
}

/// What a kernel borrows from the engine besides rows: the complete inventory.
pub trait Resources {
    /// Where the texts of the open file's cells are: its symbols, its bytes, the run's paths and the workers' texts.
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

/// What a node of the physical plan reads its rows from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum RowSource {
    /// A relation's rows, lent by the facts.
    Facts(Relation),
    /// A tree relation's rows, read from the open file's elements.
    Tree(Relation),
    /// An earlier node's materialized rows.
    Node(PlanNodeId),
    /// What the files handed over for an entry of a rule's frontier.
    Frontier(FrontierPlace),
    /// The rows a bound observation of a rule answers.
    Observation(BoundObservationPlace),
}

/// The place of a shared index among a plan's indexes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexPlace(pub(crate) usize);

/// How many nodes a plan runs.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::NodeTotal;
/// assert_eq!(NodeTotal(2), NodeTotal(2));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeTotal(pub usize);

/// How many node readers a node has, or has left in the open file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NodeReaders(pub(crate) u32);

/// Whether a node's rows are held for its readers, or passed on, chunk by chunk, to its one reader as they are made, so the
/// node never holds them: decided once by the planner, after the rewrites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeFlow {
    /// Its rows are held until its last consumer is done.
    Held,
    /// Its rows stream into its one reader, a streaming stage that reads it first.
    Streamed,
}

/// How many engine uses a node has, or has left in the open file: its entries in the program's emitted and held lists (seam
/// D's own lists), each a rule that emits it or a consumer that holds it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct EngineUses(pub(crate) u32);

/// A `hash_index` shared by every reader of one source's key: built once per file for all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyedIndex {
    /// The rows it indexes.
    pub(crate) source: RowSource,
    /// What it keys them by.
    pub(crate) key: IndexKey,
    /// The nodes that read it, in node order.
    pub(crate) readers: Vec<PlanNodeId>,
}

/// Which shared index a reader reads: the rows it indexes and what it keys them by. Two readers of one identity share one index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IndexIdentity {
    /// The rows it indexes.
    pub(crate) source: RowSource,
    /// What it keys them by.
    pub(crate) key: IndexKey,
}

/// The indexes a planning step finds, in the order it first meets them, each found again by its identity in one lookup: a plan of
/// many readers would scan them all for each reader.
#[derive(Default)]
pub(crate) struct KeyedIndexes {
    /// Every index met, with its readers in node order.
    pub(crate) indexes: Vec<KeyedIndex>,
    /// Each index's place in `indexes`, by its identity.
    pub(crate) places: hashbrown::HashMap<IndexIdentity, IndexPlace>,
}

/// Who reads a node's rows, found in one pass over every node's sources: how many reads there are, and the first.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ReadersFound {
    /// How many sources of later nodes name it, a node naming it twice counted twice.
    pub(crate) count: NodeReaders,
    /// The first source naming it: the reader and the source's place among the reader's sources.
    pub(crate) first: Option<FirstRead>,
}

/// One source naming a node: the reader, and the source's place among the reader's sources (a probe's left is first).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FirstRead {
    /// The node that reads it.
    pub(crate) reader: PlanNodeId,
    /// The source's place among the reader's sources.
    pub(crate) place: ryo_values::domain::plan_parts::types::InputNumber,
}

/// What a shared index keys its rows by: a name column's text for lookups, key columns' cells for probes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum IndexKey {
    /// One name column's text.
    Text(ColumnIndex),
    /// The cells of the key columns.
    Cells(Vec<ColumnIndex>),
}

/// The index a probe reads: built by each of its runs, or shared with every probe of its right rows and key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProbeIndex {
    /// Its own.
    Own,
    /// The shared index at this place.
    Shared(IndexPlace),
    /// The open file's tree is the index: each left row walks from its node.
    Tree,
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
    /// One row of a chain of streamed stages, read in place: its cells and what each holds.
    Row(
        &'a [Value],
        &'a [ryo_relations::domain::relations::types::CellKind],
    ),
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CellStart(pub(crate) usize);

/// What a `text_map` stage does to each row: one text kind's per-row meaning, its parts built by the kind's own lowering.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RowMap {
    /// `capture`.
    Capture(crate::domain::capture::types::CaptureParts),
    /// `replace`.
    Replace(crate::domain::replace::types::ReplaceParts),
    /// `source-matches`.
    SourceMatches(crate::domain::source_matches::types::SourceMatchesParts),
    /// `span-bounds`.
    SpanBounds(crate::domain::span_bounds::types::SpanBoundsParts),
    /// `span-between`.
    SpanBetween(crate::domain::span_between::types::SpanBetweenParts),
    /// `words`.
    Words(crate::domain::words::types::WordsParts),
    /// `rank`.
    Rank(crate::domain::rank::types::RankParts),
    /// `fits-rendered`.
    FitsRendered(crate::domain::fits_rendered::types::FitsParts),
}

/// The tree nodes whose rows of a tree relation one node reads, before any elements are lent: the rows' form and kinds, and
/// the nodes, in tree order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TreeRoots<'a> {
    /// Which rows of the tree these are.
    pub(crate) form: crate::domain::tree_cells::types::TreeForm,
    /// What each column holds.
    pub(crate) kinds: &'static [ryo_relations::domain::relations::types::CellKind],
    /// The tree nodes whose rows these are, in tree order.
    pub(crate) candidates: &'a [NodeId],
}

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
