//! One evaluation of a plan operation: the borrowed views the engine lends, the scratch it keeps for the
//! operators, and the driver port the engine implements once. Every kind's row meaning is written once, in
//! operators; the engine's physical strategies (index probes, native candidates, walks over its trees and
//! partitions) are implementations of the ports below.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::cycles::types::{CycleDriver, CycleScratch};
use crate::domain::equality_join::types::JoinDriver;
use crate::domain::ordered_match::types::OrderedMatchDriver;
use crate::domain::select::types::PredicateOutcome;
use crate::domain::set_rows::types::{FirstOccurrence, SetRowDriver};
use crate::domain::source_matches::types::{BaseLine, LineBytes, SourceBase, TextRowDriver};
use ryo_relations::domain::relations::types::{Count, Integer, NodeId, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Direction, InputNumber, LiteralText};
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::row_texts::types::CellText;
use ryo_values::domain::rows::types::{RowCount, RowIndex};

/// The rows one evaluation is given: one scheduled row of its primary input, or its complete populations,
/// which the operators read through the driver.
#[derive(Clone, Copy, Debug)]
pub enum EvaluationInput<'a> {
    /// One scheduled row.
    Row(&'a [Value]),
    /// The complete contributing populations, or the request's own source rows.
    Populations,
}

/// The source text an occurrence search was validated against, before any matcher was acquired.
#[derive(Clone, Copy, Debug)]
pub struct CheckedSource<'a> {
    /// Where the checked text starts inside the physical source.
    pub base: SourceBase,
    /// The checked text.
    pub text: CellText<'a>,
    /// The line the text starts on.
    pub line: BaseLine,
}

/// A complete reachability answer the engine already holds for one request: the partition keys and the
/// source, then each destination in typed sort order.
#[derive(Clone, Copy, Debug)]
pub struct CachedReach<'a> {
    /// The partition keys followed by the source.
    pub request: &'a [Value],
    /// The reached destinations.
    pub destinations: &'a [Value],
}

/// Plain data of one invocation, copied out of the engine's views before the driver borrows the engine.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationViews<'a> {
    /// The physical source of the file being read.
    pub source: LineBytes<'a>,
    /// The request's parameter bindings.
    pub bindings: &'a [ParameterBinding],
    /// The matcher the engine acquired for this operation.
    pub regex: Option<&'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    /// The span text an occurrence search was validated against.
    pub checked: Option<CheckedSource<'a>>,
    /// The resolved suffix an extension appends.
    pub appended: Option<&'a [Value]>,
    /// The resolved text-set layers of a rank.
    pub layers: Option<&'a [Vec<LiteralText>]>,
    /// The complete reachability answer of a logical closure join.
    pub cached_reach: Option<CachedReach<'a>>,
    /// The complete answer of one axis request, after domain membership and nearest filtering.
    pub cached_axis: Option<&'a [Value]>,
}

/// Whether a resolved cell is text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFound {
    /// The cell is text; it was appended to the caller's buffer.
    Text,
    /// The cell holds no text.
    NotText,
}

/// Whether a population row was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowRead {
    /// The row was copied into the caller's buffer.
    Read,
    /// No row is at that place.
    Missing,
}

/// Whether the engine hands the right side of a join in logical columns or in physical tree columns, which
/// the declared view remaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightColumns {
    /// The right side already answers the requested logical view.
    Logical,
    /// The right side answers physical tree columns; the view remaps them.
    Physical,
}

/// One group of a grouped population, as the engine prepared it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupSummary {
    /// The first row of the group, in arrival order.
    pub first: RowIndex,
    /// The rows the group counts.
    pub counted: Count,
    /// The digest of the group's distinct values; zero when none was asked for.
    pub digest: Integer,
}

/// A partition of a closure, by its place among the closure's partitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartitionPlace(pub usize);

/// A closure partition's sources, or the vertices one walk reached, by their place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VertexPlace(pub usize);

/// The engine's borrowed closure partitions and its reachability walk over them.
pub trait ClosureDriver {
    /// How many partitions the closure holds.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring the partitions.
    fn partitions(&mut self) -> Result<Count, PortableOperatorFault>;
    /// Copies the key of a partition into `key`.
    ///
    /// # Errors
    ///
    /// The engine retained an error reading the partition.
    fn partition_key(
        &mut self,
        partition: PartitionPlace,
        key: &mut Vec<Value>,
    ) -> Result<(), PortableOperatorFault>;
    /// How many sources a partition has.
    fn sources(&self, partition: PartitionPlace) -> Count;
    /// One source of a partition.
    fn source(&self, partition: PartitionPlace, at: VertexPlace) -> Option<Value>;
    /// Walks every vertex reachable from a source of a partition, in the walk's order.
    ///
    /// # Errors
    ///
    /// The engine retained an error walking the partition.
    fn reach(
        &mut self,
        partition: PartitionPlace,
        source: Value,
    ) -> Result<Count, PortableOperatorFault>;
    /// One vertex the last walk reached.
    fn reached(&self, at: VertexPlace) -> Option<Value>;
}

/// What the candidates of one axis walk are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisCandidates {
    /// Entities reached over an index of edges; each row appends the entity.
    Entities(Count),
    /// Elements reached over a tree; each row appends the element's node, or the declared output.
    Elements(Count),
}

/// Whether a candidate of an axis walk is admitted: an entity of the domain, an element its domain
/// predicate holds for or its dense marks hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Admission {
    /// The candidate is admitted.
    In,
    /// The candidate is not admitted.
    Out,
}

/// The engine's borrowed axis resources and its walk over them: a tree of elements, or an index of edges
/// with its domain membership.
pub trait AxisDriver {
    /// Walks in `direction` from the start the row holds in column `id`, and keeps the candidates in walk
    /// order; an axis with no resources, or a start of another file, has none.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring or walking the axis, or reading the start.
    fn walk(
        &mut self,
        direction: Direction,
        row: &[Value],
        id: ColumnIndex,
    ) -> Result<AxisCandidates, PortableOperatorFault>;
    /// Whether a candidate of the last walk is admitted.
    ///
    /// # Errors
    ///
    /// The engine retained an error evaluating the domain.
    fn admits(&mut self, at: VertexPlace) -> Result<Admission, PortableOperatorFault>;
    /// One candidate entity, or the node of one candidate element.
    fn candidate(&self, at: VertexPlace) -> Option<Value>;
    /// One right cell of a candidate element.
    fn element_cell(&self, at: VertexPlace, column: ColumnIndex) -> Option<Value>;
}

/// One edge an ordered depth-first visit reached, with its ordinal among the edges visited from its root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderedEdge {
    /// The edge row, by its place in the prepared edges.
    pub edge: RowIndex,
    /// The ordinal of the visit.
    pub ordinal: Integer,
}

/// The engine's prepared ordered depth-first graph and its walk.
pub trait DfsDriver {
    /// How many root rows the graph holds, in their prepared sorted order.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring the graph.
    fn roots(&mut self) -> Result<RowCount, PortableOperatorFault>;
    /// Copies the root row at a place of the sorted order into `row`.
    fn root(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead;
    /// Copies the root columns of the prepared graph into `columns`: the partition keys followed by the
    /// canonical root.
    fn root_columns(&self, columns: &mut Vec<ColumnIndex>);
    /// Starts a visit from a root row's partition and canonical root.
    ///
    /// # Errors
    ///
    /// The engine retained an error starting the walk.
    fn begin(&mut self, partition: &[Value], root: Value) -> Result<(), PortableOperatorFault>;
    /// The next edge of the visit, or none when it is over.
    ///
    /// # Errors
    ///
    /// The engine retained an error walking the graph.
    fn next(&mut self, partition: &[Value]) -> Result<Option<OrderedEdge>, PortableOperatorFault>;
    /// Copies an edge row into `row`.
    fn edge(&self, at: RowIndex, row: &mut Vec<Value>) -> RowRead;
}

/// The engine's side of one evaluation: delivery, the bound predicate, the invocation's distinct state,
/// symbols and populations, and the strategy ports of the kinds that read engine-prepared structures.
pub trait InvocationDriver {
    /// Delivers one output row downstream.
    ///
    /// # Errors
    ///
    /// The engine retained a downstream error; every later call stays interrupted.
    fn emit(&mut self, row: &[Value]) -> Result<(), PortableOperatorFault>;
    /// Evaluates the operation's bound predicate program over a row.
    ///
    /// # Errors
    ///
    /// The engine retained an error evaluating the predicate.
    fn holds(&mut self, row: &[Value]) -> Result<PredicateOutcome, PortableOperatorFault>;
    /// Records a row in the invocation's first-occurrence state.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no distinct state is bound, or its schema was refused.
    fn first_time(&mut self, row: &[Value]) -> Result<FirstOccurrence, PortableOperatorFault>;
    /// Counts rows the step found at one of its inputs.
    fn found(&mut self, input: InputNumber, rows: Count);
    /// Resolves a cell to its text and appends it to `target`; a cell that is not text leaves it as it was.
    fn text(&mut self, value: Value, target: &mut String) -> TextFound;
    /// Interns a text as a derived symbol of the file being read.
    fn mint(&mut self, text: CellText<'_>) -> Value;
    /// How many rows a population holds.
    ///
    /// # Errors
    ///
    /// The engine retained an error: the population is absent.
    fn population(&mut self, input: InputNumber) -> Result<RowCount, PortableOperatorFault>;
    /// Copies one row of a population into `row`.
    fn population_row(&mut self, input: InputNumber, at: RowIndex, row: &mut Vec<Value>)
    -> RowRead;
    /// How many groups the engine prepared for a grouped population.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no grouping is bound.
    fn groups(&mut self) -> Result<Count, PortableOperatorFault>;
    /// One prepared group.
    fn group(&self, at: VertexPlace) -> Option<GroupSummary>;
    /// Renders each node in the operation's syntax role and keeps the answers for `rendered`.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no semantic session, or a batch of another shape.
    fn render(&mut self, nodes: &[NodeId]) -> Result<(), PortableOperatorFault>;
    /// Copies the kept answer of one rendered node into `text`.
    fn rendered(
        &self,
        at: RowIndex,
        text: &mut String,
    ) -> crate::domain::render_batch::types::RenderKind;
    /// Whether the join's right side answers logical or physical columns.
    fn right_columns(&self) -> RightColumns;
    /// The engine's candidate source of an equality join: an index probe or native tree candidates.
    fn join(&mut self) -> &mut dyn JoinDriver;
    /// The engine's membership, distinct and delivery boundary of a set row.
    fn set_rows(&mut self) -> &mut dyn SetRowDriver;
    /// The engine's symbol and delivery boundary of an occurrence search.
    fn text_rows(&mut self) -> &mut dyn TextRowDriver;
    /// The engine's prepared ordered keys of both inputs.
    ///
    /// # Errors
    ///
    /// The engine retained an error: an input or the ordered keys are absent.
    fn ordered_match(&mut self) -> Result<&mut dyn OrderedMatchDriver, PortableOperatorFault>;
    /// The engine's prepared adjacency of a cycle search.
    ///
    /// # Errors
    ///
    /// The engine retained an error: no adjacency is bound.
    fn cycles(&mut self) -> Result<&dyn CycleDriver, PortableOperatorFault>;
    /// The engine's closure partitions and walk.
    fn closure(&mut self) -> &mut dyn ClosureDriver;
    /// The engine's axis resources and walk.
    fn axis(&mut self) -> &mut dyn AxisDriver;
    /// The engine's ordered depth-first graph and walk.
    fn dfs(&mut self) -> &mut dyn DfsDriver;
}

/// The buffers one evaluation fills, kept by the engine between invocations so they keep their capacity.
#[derive(Debug, Default)]
pub struct EvaluationScratch {
    /// The row being answered.
    pub row: Vec<Value>,
    /// A second row: a projection, a prefix, a population row.
    pub other: Vec<Value>,
    /// Captured or derived values.
    pub captured: Vec<Value>,
    /// A key.
    pub key: Vec<Value>,
    /// A partition's key.
    pub partition: Vec<Value>,
    /// A resolved or built text.
    pub text: String,
    /// The word being split.
    pub word: String,
    /// The text a replacement builds.
    pub replaced: String,
    /// The slot ranges of a path capture.
    pub slots: Vec<std::ops::Range<usize>>,
    /// Columns of a prepared structure.
    pub columns: Vec<ColumnIndex>,
    /// The node of each render request.
    pub nodes: Vec<NodeId>,
    /// What the provider answered for each render request.
    pub rendered: Vec<crate::domain::render_batch::types::RenderKind>,
    /// The work arrays of a cycle search.
    pub cycles: CycleScratch,
}
