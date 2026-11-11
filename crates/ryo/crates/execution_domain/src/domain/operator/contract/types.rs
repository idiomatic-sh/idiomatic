// ignore: idiomatic.arch/file-length - the prepared contract and the borrowed ports an operator reads it through, each port method stating its errors; splitting the contract from its ports would split one interface
//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{types::PredicateProgram, types::ValidatedSource};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::PrimitiveSemantics;
use crate::domain::operator::contract::presence::{types::PresenceMask, types::RowFailure};
use crate::domain::table_reach::borrowed::types::IndexedRows;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

/// The finite existing semantic input shapes; dependency acquisition is engine work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// One row and immutable side resources, preserving immediate emitter failure.
    Row {
        /// The input whose rows trigger this mode.
        primary: ryo_values::domain::plan_parts::types::InputNumber,
    },
    /// The complete contributing population is semantic input.
    Views,
    /// Rows or Hosted is supplied by the request owner, not a fake ordinary input.
    RequestSource,
}

/// A closed dependency order and lazy gate, interpreted by the engine before evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DemandOrder {
    /// No upstream rows are needed.
    None,
    /// Read the sole upstream input.
    Unary,
    /// Read left input before right input.
    LeftThenRight,
}

/// Resource declarations are semantic needs, never primitive-controlled cache policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceNeeds {
    /// No acquired side resource is needed.
    None,
    /// Prepared predicate evaluation.
    Predicates,
    /// Distinct row membership.
    DistinctRows,
    /// Key membership index.
    KeyMembership,
    /// Join key index.
    JoinKeys,
    /// Group counts.
    GroupCounts,
    /// Group digest state.
    GroupDigest,
    /// Literal matching patterns.
    LiteralPatterns,
    /// Ordered key index.
    OrderedKeys,
    /// Traversal adjacency.
    Adjacency,
    /// Adjacency with stable edge ordering.
    OrderedAdjacency,
}

/// The kind of symbolic mutation that is observable in a successful invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effects {
    /// The result changes no symbolic state.
    Pure,
    /// The result writes derived text.
    DerivedText,
    /// The result reads or emits physical source text.
    PhysicalSourceText,
    /// The result depends on request-provided source.
    RequestSource,
}

/// One prepared semantic dependency and input contract for every output consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticContract {
    /// The input shape the operation reads.
    pub shape: InputMode,
    /// The order the engine reads the operation's upstream inputs in.
    pub demand: DemandOrder,
    /// Where a missing side resource is refused: before the rows or at the first applicable row.
    pub acquisition: AcquisitionBoundary,
    /// The inputs whose rows trigger an evaluation.
    pub row_inputs: RowInputMask,
    /// The order the engine checks the operation's preflight conditions in.
    pub preflight: PreflightOrder,
}

/// What an operation declares of its semantics: its input contract, its effects and the
/// inputs whose absence of rows it trusts.
#[derive(Debug, Clone, Copy)]
pub struct OperationSemantics {
    /// The operation's input contract.
    pub contract: SemanticContract,
    /// Whether the operation is pure or has effects.
    pub effects: Effects,
    /// The inputs whose absence of rows the operation's answer depends on.
    pub absence: AbsenceMask,
    /// The inputs that must have rows for the answer to have any.
    pub presence: PresenceMask,
}

/// The operands, by their place, whose absence of rows an operator trusts: the bit of an operand is `1 << place`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AbsenceMask(pub u8);

/// Inputs whose rows may trigger evaluation in this mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowInputMask(pub u8);

/// Resource acquisition preserves the old applicable-input refusal boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionBoundary {
    /// Before reading input rows.
    BeforeRows,
    /// At the first applicable input row.
    FirstRow,
}

/// Physical source validation precedes resource acquisition for SourceMatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceGate {
    /// No physical source check.
    None,
    /// Check a source text column.
    CheckedText {
        /// Text column to check.
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// Check an in-file span and optional line base.
    CheckedSpan {
        /// Span column to check.
        span: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Optional base-line column.
        base_line: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
    },
}

/// Whether collected preflight validates schema before resource acquisition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreflightOrder {
    /// Check the complete schema before acquiring resources.
    SchemaFirst,
    /// Acquire resources before validating the complete schema.
    ResourcesFirst,
}

/// How long an operator's logical state lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceLifetime {
    /// The state lives for the whole invocation.
    Invocation,
    /// The state is reset at each primary input row.
    PrimaryRow,
}
/// The logical state an operation keeps between rows, which the invocation owner interprets.
#[derive(Clone, Debug)]
pub enum WorkspaceProgram {
    /// The operation keeps no state.
    None,
    /// The operation keeps the first occurrence of each row.
    FirstOccurrences {
        /// How long the remembered rows live.
        lifetime: WorkspaceLifetime,
        /// The kinds of the remembered rows, or why they could not be known.
        kinds: Result<Vec<CellKind>, OperatingError>,
    },
}

/// Owned by one request and aligned with the compiled operation.
#[derive(Clone, Debug)]
pub struct OperatorContract {
    /// The derived views the operation reads, one slot each.
    pub derived: crate::domain::derived_views::types::contract::value::DerivedSlots,
    /// The kind of the operation.
    pub kind: OperationKind,
    /// The operation's inputs.
    pub inputs: Inputs,
    /// The cell kinds of each input, in input order.
    pub input_kinds: Vec<Vec<CellKind>>,
    /// The operation's semantic input contract.
    pub semantics: SemanticContract,
    /// Metadata errors remain deferred to the declared execution boundary.
    pub schema: Result<Vec<CellKind>, OperatingError>,
    /// Payload-independent collected layout; row-lazy refusals are not raised by it.
    pub output_kinds: Result<Vec<CellKind>, OperatingError>,
    /// Input-position bitset: absence-dependent outputs require these populations complete.
    pub absence_inputs: AbsenceMask,
    /// The logical state the operation keeps between rows.
    pub workspace: WorkspaceProgram,
    /// The side resources the operation acquires before it evaluates.
    pub resources: super::resources::types::ResourceProgram,
    /// The results the operation answers when an input is empty.
    pub empty_results: Vec<EmptyResult>,
    /// How the operation's rows are checked against the source text.
    pub source_gate: SourceGate,
    /// Which inputs' rows can make the operation fail, decided from the prepared operation and its input kinds.
    pub failure: RowFailure,
    /// The error every row of the operation is refused with, when the compiled operation is malformed.
    pub row_error: Option<OperatingError>,
    /// The literal pattern the operation matches, when it has one.
    pub literal_pattern: Option<ryo_values::domain::plan_parts::types::LiteralText>,
    /// The predicate program bound to the operation.
    pub predicates: PreparedPredicates,
    pub effects: Effects,
    pub reuse: ReuseScope,
    /// Exact typed fields, not a hash-only or Debug-text key.
    pub identity: Operation,
}

/// Read-only data borrowed from the application-owned invocation. No owner,
/// scheduler, acquisition or release operation crosses this interface.
pub trait PrimitiveResources {
    /// Checks that every declared resource was acquired, in the common lease's deferred error order.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of the first declared resource that cannot be viewed.
    fn validate_declared(&self) -> Result<(), OperatingError>;
    /// The key index of the operation's input, when one was acquired.
    fn index(&self) -> Option<IndexedRows<'_>>;
    /// The rows appended to the input, when the operation appends any.
    fn appended(&self) -> Option<&[Value]>;
    /// The literal layers of a rank, when the operation ranks.
    fn rank_layers(&self) -> Option<&[Vec<ryo_values::domain::plan_parts::types::LiteralText>]>;
    /// The prepared axis of the operation, when it walks one.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of an acquired axis that cannot be viewed.
    fn axis(&self) -> Result<Option<super::views::types::PreparedAxis<'_>>, OperatingError>;
    /// The native join lookup, while its derived acquisition is held.
    fn native_join(&self) -> Option<super::views::types::NativeJoin<'_>>;
    /// The prepared depth-first search, when the operation runs one.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of an acquired search that cannot be viewed.
    fn dfs(&self) -> Result<Option<&ryo_derived::domain::dfs::types::PreparedDfs>, OperatingError>;
    /// The cycle adjacency, when the operation reads one.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of an acquired adjacency that cannot be viewed.
    fn cycles(
        &self,
    ) -> Result<Option<&ryo_derived::domain::cycle_adjacency::types::CycleGraph>, OperatingError>;
    /// The closure partitions the operation reads.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of a current partition that cannot be read.
    fn closure(&self) -> Result<super::views::types::ClosureViews<'_>, OperatingError>;
    /// The ordered matching of two ordered inputs, when the operation pairs them.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] of an acquired matching that cannot be viewed.
    fn ordered(
        &self,
    ) -> Result<Option<&ryo_derived::domain::ordered::types::OrderedMatching>, OperatingError>;
    /// The groups of the operation's input, when it aggregates.
    fn groups(&self) -> Option<&[crate::domain::table_groups::types::Group]>;
}

/// Only row-varying context is assembled at row delivery. Acquired resources
/// remain in the invocation and are reached through a borrowed reader.
pub struct PrimitiveViews<'a> {
    /// Whether the operation runs after the files of the request.
    pub phase: crate::domain::operating::types::AfterTheFiles,
    /// The resources acquired by the invocation, read through a borrowed reader.
    pub resources: &'a dyn PrimitiveResources,
    /// The semantic answers the operation may look up, when a semantic session is attached.
    pub semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    /// The path of each file of the run, by file number.
    pub paths: &'a [ryo_language::domain::files::types::FilePath],
    /// The text each worker made, by worker number.
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    /// The bytes of the file being read.
    pub source: SourceView<'a>,
    /// The request's parameter bindings.
    pub bindings: &'a [ParameterBinding],
    /// The source text checked against the operation's source gate, when it has one.
    pub validated_source: Option<ValidatedSource<'a>>,
    /// The compiled regular expression the operation matches, when it has one.
    pub regex: Option<&'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    /// The reach answered from a cached graph, when one was acquired.
    pub cached_reach: Option<super::graph_view::types::ReachResult<'a>>,
    /// The axis answered from a cached graph, when one was acquired.
    pub cached_axis: Option<super::graph_view::types::AxisResult<'a>>,
}

/// Allocation slots owned once by the application invocation. A primitive only
/// receives a reborrow; no cache, owner or generation identity is reachable.
#[derive(Debug)]
pub struct PrimitiveWorkspace {
    /// The buffers the operators fill while they evaluate a kind.
    pub evaluation: ryo_operators::domain::evaluation::types::EvaluationScratch,
    /// The work of the axis walk, when the operation walks an axis.
    pub axis: Option<AxisWorkspace>,
    pub positions: Vec<ryo_relations::domain::relations::types::Position>,
    /// The work of the ordered depth-first search, when the operation runs one.
    pub dfs: Option<DfsWorkspace>,
    /// The walk of a closure traversal, when the operation runs one.
    pub closure_walk: Option<crate::domain::table_reach::types::Walk>,
    /// The first-occurrence state, when the operation keeps distinct rows.
    pub distinct: Option<DistinctWorkspace>,
}
/// The rows found at each of the three inputs of a counted step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkTotals(pub [FoundRows; 3]);

/// How many rows a counted step found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoundRows(pub u64);

/// The engine constructs and releases this borrow; a primitive owns no service state.
/// Borrowed logical work counters; storage belongs to the invocation owner, and no caller outside
/// the domain sets them.
///
/// ```compile_fail,E0423
/// use ryo_execution_domain::domain::operator::contract::types::WorkCounters;
/// let forged = WorkCounters(Default::default());
/// ```
#[derive(Default, Debug)]
pub struct WorkCounters(pub(super) [std::cell::Cell<u64>; 3]);
/// The borrowed context one primitive evaluation reaches its capabilities through. Only the
/// engine leases one: no caller outside the domain reads its predicate, semantics or counters.
///
/// ```compile_fail,E0451
/// use ryo_execution_domain::domain::operator::contract::types::PrimitiveLease;
/// fn semantics(lease: PrimitiveLease<'_>) {
///     let PrimitiveLease { semantics, .. } = lease;
/// }
/// ```
pub struct PrimitiveLease<'a> {
    pub(crate) views: PrimitiveViews<'a>,
    pub(crate) workspace: &'a mut PrimitiveWorkspace,
    /// The symbols the evaluation interns derived text into.
    pub derived: QuerySymbols<'a>,
    pub(super) predicate: &'a mut dyn PredicateProgram,
    pub(super) semantics: &'a dyn PrimitiveSemantics,
    pub(super) work: &'a WorkCounters,
}

/// A narrow lease reborrow preserves source/derived symbols while downstream rows run.
pub trait RowSink {
    /// Delivers one row the primitive emits, with the lease reborrowed for the rows downstream.
    ///
    /// # Errors
    ///
    /// The [`OperatingError`] a downstream consumer of the row returns; the primitive stops at it.
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError>;
}

/// First-occurrence row semantics, bounded and released by the engine invocation.
#[derive(Debug)]
pub struct DistinctWorkspace {
    /// Whether the remembered rows' schema was admitted, or why not.
    pub schema: Result<(), OperatingError>,
    /// The rows seen first, in the order they came.
    pub rows: ryo_values::domain::rows::types::Table,
    /// The rows already seen, by their cells.
    pub seen: hashbrown::HashTable<ryo_values::domain::rows::types::RowIndex>,
}

/// How many operations a request prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationCount(pub usize);

/// Exact semantic program bound once; a primitive cannot substitute comparisons.
#[derive(Clone, Debug)]
pub struct PreparedPredicates {
    /// The comparisons the predicate evaluates.
    pub comparisons: Vec<ryo_values::domain::plan_parts::types::PlanComparison>,
    /// The cell kinds the comparisons read.
    pub kinds: Vec<CellKind>,
}

/// Binding/effect identity constrains sharing, never engine admission.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::operator::contract::types::ReuseScope;
/// assert_ne!(ReuseScope::Request, ReuseScope::Rule);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReuseScope {
    /// Shared across the rules of one request.
    Request,
    /// Shared within one rule only.
    Rule,
}

/// Which representation an empty result is answered for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyRepresentation {
    /// Any representation.
    Any,
    /// Only a computed table.
    Computed,
}
/// The result an operation answers when one of its inputs is empty.
#[derive(Clone, Debug)]
pub struct EmptyResult {
    /// The input whose emptiness decides the result.
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The representation the empty result is answered for.
    pub representation: EmptyRepresentation,
    /// The kinds of the empty result's columns, or why they could not be known.
    pub kinds: Result<Vec<CellKind>, OperatingError>,
}

/// The work of one invocation of an axis walk.
#[derive(Debug, Default)]
pub struct AxisWorkspace {
    /// The walk the axis performs.
    pub walk: crate::domain::table_reach::types::Walk,
    pub found: Vec<ryo_relations::domain::relations::types::Position>,
}

/// The vertices one invocation of the ordered depth-first search has expanded.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExpandedCount(pub usize);

/// The edges one invocation of the ordered depth-first search has examined.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExaminedCount(pub usize);

/// The work of one invocation of the ordered depth-first search, with what its walks expanded and examined.
#[derive(Debug, Default)]
pub struct DfsWorkspace {
    /// The ordered walk the search performs.
    pub walk: crate::domain::table_reach::types::OrderedWalk,
    /// How many vertices the walk expanded.
    pub expanded: ExpandedCount,
    /// How many edges the walk examined.
    pub examined: ExaminedCount,
}

impl Default for PrimitiveWorkspace {
    fn default() -> Self {
        Self {
            evaluation: Default::default(),
            axis: Some(Default::default()),
            positions: Default::default(),
            dfs: Some(Default::default()),
            closure_walk: Some(Default::default()),
            distinct: Some(Default::default()),
        }
    }
}

impl Default for DistinctWorkspace {
    fn default() -> Self {
        Self {
            rows: Default::default(),
            seen: Default::default(),
            schema: Err(OperatingError::NotRun),
        }
    }
}
