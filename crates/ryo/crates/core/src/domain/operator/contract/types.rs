//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{BorrowedRows, IndexedRows};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_texts::types::SourceView;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::{Operation, OperationKind};

/// This affects error/publication timing, not which primitive implementation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationMode {
    /// Emit rows as they are computed.
    Streaming,
    /// Consume a complete input before evaluating.
    Collected,
}

/// The finite existing semantic input shapes; dependency acquisition is engine work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputShape {
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
pub(crate) enum DemandOrder {
    /// No upstream rows are needed.
    None,
    /// Read the sole upstream input.
    Unary,
    /// Read left input before right input.
    LeftThenRight,
    /// Acquire join input at its declared transport boundary.
    JoinTransport,
    /// Acquire axis input at its declared transport boundary.
    AxisTransport,
    /// Skip right input when the left input proves empty.
    LeftThenRightUnlessEmptyLeft,
}

/// A complete-view algorithm may fail before any output is observable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Publication {
    /// Emit successful rows incrementally.
    Incremental,
    /// Publish rows only after complete success.
    AfterSuccess,
}

/// Engine-owned lifetime of semantic first-occurrence work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetScope {
    /// No first-occurrence state is required.
    None,
    /// Share first-occurrence work for one invocation.
    Invocation,
    /// Restart first-occurrence work for each primary row.
    PrimaryRow,
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
pub(crate) enum Effects {
    /// The result changes no symbolic state.
    Pure,
    /// The result writes derived text.
    DerivedText,
    /// The result reads or emits physical source text.
    PhysicalSourceText,
    /// The result depends on request-provided source.
    RequestSource,
}

/// Mode-specific failure and demand behavior is prepared once, never per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ModeContract {
    /// Semantic form of inputs seen by the operator.
    pub(crate) shape: InputShape,
    /// Upstream demand order.
    pub(crate) demand: DemandOrder,
    /// Visibility boundary for ordinary evaluation.
    pub(crate) publication: Publication,
    /// Visibility boundary when input is held.
    pub(crate) held_publication: Publication,
    /// When schema validation is observable.
    pub(crate) schema_boundary: SchemaBoundary,
    /// When acquired resources are observable.
    pub(crate) acquisition: AcquisitionBoundary,
    /// Input positions read in row mode.
    pub(crate) row_inputs: RowInputMask,
    /// Whether the mode streams or collects.
    pub(crate) evaluation: EvaluationMode,
    /// Order of collected-mode preflight checks.
    pub(crate) preflight: PreflightOrder,
}

/// Inputs whose rows may trigger evaluation in this mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RowInputMask(pub u8);

/// Resource acquisition preserves the old applicable-input refusal boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AcquisitionBoundary {
    /// Before reading input rows.
    BeforeRows,
    /// At the first applicable input row.
    FirstRow,
}

/// Collection validates the complete schema before reading; streaming row transforms
/// retain their first-applicable-row error boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaBoundary {
    /// Before upstream demand.
    BeforeDemand,
    /// At the first row that uses the schema.
    FirstApplicableRow,
}

/// Physical source validation precedes resource acquisition for SourceMatches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceGate {
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
pub(crate) enum PreflightOrder {
    /// Check the complete schema before acquiring resources.
    SchemaFirst,
    /// Acquire resources before validating the complete schema.
    ResourcesFirst,
}

/// Owned by one request and aligned with the compiled operation.
#[derive(Clone, Debug)]
pub(crate) struct OperatorContract {
    pub derived: crate::domain::derived_views::types::DerivedSlots,
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
    /// File-scoped semantic demand, borrowed only for this invocation.
    pub semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
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
    pub positions: &'a mut Vec<ryo_relations::domain::relations::types::Position>,
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
    semantics: &'a dyn PrimitiveSemantics,
}

/// Borrowed portable primitive meanings; the application supplies the implementation.
/// No outbound adapter, cache owner or scheduler is reachable from a primitive.
pub(crate) trait PrimitiveSemantics: Sync {
    fn operators(
        &self,
        call: ryo_operators::domain::contract::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >;
}

fn portable_fault(
    error: ryo_operators::domain::contract::types::PortableOperatorFault,
) -> OperatingError {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    match error {
        PortableOperatorFault::NoSuchColumn { index } => {
            crate::domain::table_model::errors::TableModelError::NoSuchColumn { index }.into()
        }
        PortableOperatorFault::ShapeMismatch => OperatingError::ShapeMismatch,
        PortableOperatorFault::InvalidSpanBounds => OperatingError::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan => OperatingError::InvalidSpan,
    }
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
    pub selection_plans: std::sync::Arc<Vec<crate::domain::stored_selection::types::PreparedPlan>>,
    pub selection_views:
        std::sync::Arc<crate::domain::stored_selection::types::PreparedSelectionViews>,
    pub rules: Vec<Vec<std::sync::Arc<OperatorContract>>>,
    pub preparations: usize,
    pub physical: Vec<
        Vec<
            crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        >,
    >,
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
        semantics: &'a dyn PrimitiveSemantics,
    ) -> Self {
        Self {
            views,
            workspace,
            derived,
            predicate,
            semantics,
        }
    }
    pub(crate) fn project(
        &self,
        row: &[Value],
        columns: &[ryo_values::domain::plan_parts::types::ColumnIndex],
        projected: &mut Vec<Value>,
    ) -> Result<(), OperatingError> {
        match self.semantics.operators(
            ryo_operators::domain::contract::types::OperatorCall::Project {
                row,
                columns,
                target: projected,
            },
        ) {
            Ok(ryo_operators::domain::contract::types::OperatorAnswer::Written) => Ok(()),
            Err(ryo_operators::domain::contract::types::PortableOperatorFault::NoSuchColumn {
                index,
            }) => Err(
                crate::domain::table_model::errors::TableModelError::NoSuchColumn { index }.into(),
            ),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn append(
        &self,
        row: &[Value],
        appended: &[Value],
        joined: &mut Vec<Value>,
    ) -> Result<(), OperatingError> {
        match self.semantics.operators(
            ryo_operators::domain::contract::types::OperatorCall::Append {
                row,
                appended,
                target: joined,
            },
        ) {
            Ok(ryo_operators::domain::contract::types::OperatorAnswer::Written) => Ok(()),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn span_bounds(
        &self,
        row: &[Value],
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Result<[Value; 2], OperatingError> {
        use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall};
        match self
            .semantics
            .operators(OperatorCall::SpanBounds { row, column })
        {
            Ok(OperatorAnswer::SpanBounds(bounds)) => Ok(bounds),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
        }
    }
    pub(crate) fn span_between(
        &self,
        row: &[Value],
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        source_len: usize,
    ) -> Result<Value, OperatingError> {
        use ryo_operators::domain::contract::types::{
            OperatorAnswer, OperatorCall, SourceByteLength,
        };
        match self.semantics.operators(OperatorCall::SpanBetween {
            row,
            start,
            end,
            source_len: SourceByteLength(source_len),
        }) {
            Ok(OperatorAnswer::SpanBetween(span)) => Ok(span),
            Err(error) => Err(portable_fault(error)),
            _ => Err(OperatingError::WrongOperation),
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
