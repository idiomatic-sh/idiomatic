//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{BorrowedRows, IndexedRows};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::types::{CellKind, Value};
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// This affects error/publication timing, not which primitive implementation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationMode {
    Streaming,
    Collected,
}

/// The finite existing semantic input shapes; dependency acquisition is engine work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputShape {
    /// One row and immutable side resources, preserving immediate emitter failure.
    Row { primary: usize },
    /// The complete contributing population is semantic input.
    Views,
    /// Rows or Hosted is supplied by the request owner, not a fake ordinary input.
    RequestSource,
}

/// A closed dependency order and lazy gate, interpreted by the engine before evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DemandOrder {
    None,
    Unary,
    LeftThenRight,
    JoinTransport,
    AxisTransport,
    LeftThenRightUnlessEmptyLeft,
}

/// A complete-view algorithm may fail before any output is observable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Publication {
    Incremental,
    AfterSuccess,
}

/// Engine-owned lifetime of semantic first-occurrence work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetScope {
    None,
    Invocation,
    PrimaryRow,
}

/// Resource declarations are semantic needs, never primitive-controlled cache policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceNeeds {
    None,
    Predicates,
    DistinctRows,
    KeyMembership,
    JoinKeys,
    GroupCounts,
    GroupDigest,
    LiteralPatterns,
    OrderedKeys,
    Adjacency,
    OrderedAdjacency,
}

/// The kind of symbolic mutation that is observable in a successful invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effects {
    Pure,
    DerivedText,
    PhysicalSourceText,
    RequestSource,
}

/// Mode-specific failure and demand behavior is prepared once, never per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModeContract {
    pub shape: InputShape,
    pub demand: DemandOrder,
    pub publication: Publication,
    pub held_publication: Publication,
    pub schema_boundary: SchemaBoundary,
    pub acquisition: AcquisitionBoundary,
    pub row_inputs: u8,
    pub evaluation: EvaluationMode,
    pub preflight: PreflightOrder,
}

/// Resource acquisition preserves the old applicable-input refusal boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcquisitionBoundary {
    BeforeRows,
    FirstRow,
}

/// Collection validates the complete schema before reading; streaming row transforms
/// retain their first-applicable-row error boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaBoundary {
    BeforeDemand,
    FirstApplicableRow,
}

/// Physical source validation precedes resource acquisition for SourceMatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceGate {
    None,
    CheckedText {
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    CheckedSpan {
        span: ryo_values::domain::plan_parts::types::ColumnIndex,
        base_line: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
    },
}

/// Owned by one request and aligned with the compiled operation.
#[derive(Clone, Debug)]
pub(crate) struct OperatorContract {
    pub kind: OperationKind,
    pub inputs: Inputs,
    pub input_kinds: Vec<Vec<CellKind>>,
    pub external_source: bool,
    pub streaming: ModeContract,
    pub collected: ModeContract,
    /// Metadata errors remain deferred to the declared execution boundary.
    pub schema: Result<Vec<CellKind>, OperatingError>,
    /// Payload-independent collected layout; row-lazy refusals are not raised by it.
    pub collected_kinds: Result<Vec<CellKind>, OperatingError>,
    /// Input-position bitset: absence-dependent outputs require these populations complete.
    pub absence_inputs: u8,
    pub resources: super::resources::ResourceProgram,
    pub collected_resources: super::resources::ResourceProgram,
    pub collected_empty: Vec<EmptyResult>,
    pub set_scope: SetScope,
    pub aggregation: Option<AggregationProgram>,
    pub set_kinds: Vec<CellKind>,
    pub source_gate: SourceGate,
    pub row_error: Option<OperatingError>,
    pub literal_pattern: Option<ryo_values::domain::plan_parts::types::LiteralText>,
    pub predicates: PreparedPredicates,
    pub effects: Effects,
    pub reuse: ReuseScope,
    /// Exact typed fields, not a hash-only or Debug-text key.
    pub identity: Operation,
}

/// Immutable inputs only. No Feed, callback, scheduler or owned/retained Arc is supplied.
#[derive(Clone, Copy)]
pub(crate) enum PreparedInputs<'a> {
    Row {
        input: InputPlace,
        row: &'a [Value],
        sides: &'a [BorrowedRows<'a>],
    },
    Views(&'a [BorrowedRows<'a>]),
    RequestSource(&'a BorrowedRows<'a>),
}

/// Immutable source/index data selected and acquired by the common engine.
pub(crate) struct PrimitiveViews<'a> {
    pub paths: &'a [ryo_language::domain::files::types::FilePath],
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    pub source: SourceView<'a>,
    pub bindings: &'a [ParameterBinding],
    pub indices: &'a [IndexedRows<'a>],
    /// Already acquired by the engine at the declared lazy error boundary.
    pub validated_source: Option<
        crate::domain::operator::collections::operators::source_matches::source::ValidatedSource<
            'a,
        >,
    >,
    pub regex: Option<&'a regex_automata::meta::Regex>,
    /// Resolved once at the first applicable row (or collected preflight for Extend).
    pub appended: Option<&'a [Value]>,
    pub rank_layers: Option<&'a [Vec<ryo_values::domain::plan_parts::types::LiteralText>]>,
    pub axis: Option<&'a super::axis::PreparedAxis<'a>>,
    pub native_join: Option<super::join::NativeJoin<'a>>,
    pub cached_reach: Option<super::graph_view::ReachResult<'a>>,
    pub cached_axis: Option<super::graph_view::AxisResult<'a>>,
    pub dfs: Option<&'a super::graph::PreparedDfs>,
    pub cycles: Option<&'a super::graph::CycleGraph>,
    pub closure: &'a [super::graph::ClosurePartition],
    pub ordered: Option<&'a super::graph::OrderedMatching>,
    pub groups: Option<&'a [crate::domain::table_groups::types::Group]>,
}

/// Mutable algorithm work is invocation-bounded and supplied as borrowed slices.
/// No cache owner, growable cross-invocation memo or generation identity is reachable.
pub(crate) struct PrimitiveWorkspace<'a> {
    pub row: &'a mut Vec<Value>,
    pub other: &'a mut Vec<Value>,
    pub text: &'a mut String,
    pub word: &'a mut crate::domain::table_scratch::types::ScratchWord,
    pub stretches: &'a mut crate::domain::table_scratch::types::Stretches,
    pub captured: &'a mut Vec<Value>,
    pub axis: Option<&'a mut super::axis::AxisWorkspace>,
    pub positions: &'a mut Vec<ryo_language::domain::relations::types::Position>,
    pub dfs: Option<&'a mut super::graph::DfsWorkspace>,
    pub cycles: Option<&'a mut super::graph::CycleMarks>,
    pub closure_walk: Option<&'a mut crate::domain::table_reach::types::Walk>,
    pub distinct: Option<DistinctWorkspace<'a>>,
}

/// The engine constructs and releases this borrow; a primitive owns no service state.
pub(crate) struct PrimitiveLease<'a> {
    pub views: PrimitiveViews<'a>,
    pub workspace: PrimitiveWorkspace<'a>,
    pub derived: QuerySymbols<'a>,
    predicate: &'a mut dyn PredicateProgram,
}

/// A narrow lease reborrow preserves source/derived symbols while downstream rows run.
pub(crate) trait RowSink {
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError>;
}

/// First-occurrence row semantics, bounded and released by the engine invocation.
pub(crate) struct DistinctWorkspace<'a> {
    pub rows: &'a mut crate::domain::table_model::types::Table,
    pub seen: &'a mut hashbrown::HashTable<crate::domain::table_cells::types::RowIndex>,
}

/// Prepared metadata belongs to the request; workers only borrow this immutable value.
#[derive(Debug)]
pub(crate) struct RequestOperators {
    pub rules: Vec<Vec<std::sync::Arc<OperatorContract>>>,
    pub preparations: usize,
}

/// The evaluator is bound to the prepared contract's exact ordered program.
/// Its implementation is engine-private; this interface cannot choose predicates,
/// schemas, cache keys or upstream scheduling.
pub(crate) trait PredicateProgram {
    fn holds(
        &mut self,
        row: &[Value],
        symbols: &ryo_language::domain::symbols::types::Symbols,
    ) -> Result<crate::domain::table_model::types::Verdict, OperatingError>;
}
impl<'a> PrimitiveLease<'a> {
    pub(crate) fn new(
        views: PrimitiveViews<'a>,
        workspace: PrimitiveWorkspace<'a>,
        derived: QuerySymbols<'a>,
        predicate: &'a mut dyn PredicateProgram,
    ) -> Self {
        Self {
            views,
            workspace,
            derived,
            predicate,
        }
    }
    /// Only the bound semantic program can be requested by a primitive.
    pub(crate) fn holds(
        &mut self,
        row: &[Value],
    ) -> Result<crate::domain::table_model::types::Verdict, OperatingError> {
        self.predicate.holds(row, self.derived.as_ref())
    }
}

/// Exact semantic program bound once; a primitive cannot substitute comparisons.
#[derive(Clone, Debug)]
pub(crate) struct PreparedPredicates {
    pub comparisons: Vec<ryo_values::domain::plan_parts::types::Comparison>,
    pub kinds: Vec<CellKind>,
}

/// Finite reduction requirement, bound once; no runtime scheduling choices.
#[derive(Clone, Debug)]
pub(crate) enum AggregationProgram {
    Count {
        keys: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
        distinct: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
    },
    Digest {
        keys: Vec<ryo_values::domain::plan_parts::types::ColumnIndex>,
        over: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
}

/// Binding/effect identity constrains sharing, never engine admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReuseScope {
    Request,
    Rule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreflightOrder {
    SchemaFirst,
    ResourcesFirst,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyRepresentation {
    Any,
    Computed,
}
#[derive(Clone, Debug)]
pub(crate) struct EmptyResult {
    pub input: usize,
    pub representation: EmptyRepresentation,
    pub kinds: Result<Vec<CellKind>, OperatingError>,
}
