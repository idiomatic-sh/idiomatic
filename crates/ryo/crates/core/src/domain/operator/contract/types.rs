//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{BorrowedRows, IndexedRows};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use crate::domain::table_model::types::ParameterBinding;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::{CellKind, Value};
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

/// Read-only data borrowed from the application-owned invocation. No owner,
/// scheduler, acquisition or release operation crosses this interface.
pub(crate) trait PrimitiveResources {
    fn validate_declared(&self) -> Result<(), OperatingError>;
    fn index(&self) -> Option<IndexedRows<'_>>;
    fn appended(&self) -> Option<&[Value]>;
    fn rank_layers(&self) -> Option<&[Vec<ryo_values::domain::plan_parts::types::LiteralText>]>;
    fn axis(&self) -> Result<Option<super::axis::PreparedAxis<'_>>, OperatingError>;
    fn native_join(&self) -> Option<super::join::NativeJoin<'_>>;
    fn dfs(
        &self,
    ) -> Result<Option<&ryo_derived::domain::contract::types::PreparedDfs>, OperatingError>;
    fn cycles(
        &self,
    ) -> Result<Option<&ryo_derived::domain::contract::types::CycleGraph>, OperatingError>;
    fn closure(&self) -> super::views::ClosureViews<'_>;
    fn ordered(
        &self,
    ) -> Result<Option<&ryo_derived::domain::contract::types::OrderedMatching>, OperatingError>;
    fn groups(&self) -> Option<&[crate::domain::table_groups::types::Group]>;
}

/// Only row-varying context is assembled at row delivery. Acquired resources
/// remain in the invocation and are reached through a borrowed reader.
pub(crate) struct PrimitiveViews<'a> {
    pub resources: &'a dyn PrimitiveResources,
    pub semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    pub paths: &'a [ryo_language::domain::files::types::FilePath],
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    pub source: SourceView<'a>,
    pub bindings: &'a [ParameterBinding],
    pub validated_source: Option<
        crate::domain::operator::collections::operators::source_matches::source::ValidatedSource<
            'a,
        >,
    >,
    pub regex: Option<&'a regex_automata::meta::Regex>,
    pub cached_reach: Option<super::graph_view::ReachResult<'a>>,
    pub cached_axis: Option<super::graph_view::AxisResult<'a>>,
}

/// Allocation slots owned once by the application invocation. A primitive only
/// receives a reborrow; no cache, owner or generation identity is reachable.
pub(crate) struct PrimitiveWorkspace {
    pub row: Vec<Value>,
    pub other: Vec<Value>,
    pub text: String,
    pub word: crate::domain::table_scratch::types::ScratchWord,
    pub stretches: crate::domain::table_scratch::types::Stretches,
    pub captured: Vec<Value>,
    pub axis: Option<super::axis::AxisWorkspace>,
    pub positions: Vec<ryo_relations::domain::relations::types::Position>,
    pub dfs: Option<super::graph::DfsWorkspace>,
    pub cycles: Option<super::graph::CycleMarks>,
    pub closure_walk: Option<crate::domain::table_reach::types::Walk>,
    pub distinct: Option<DistinctWorkspace>,
}
impl Default for PrimitiveWorkspace {
    fn default() -> Self {
        Self {
            row: Default::default(),
            other: Default::default(),
            text: Default::default(),
            word: Default::default(),
            stretches: Default::default(),
            captured: Default::default(),
            axis: Some(Default::default()),
            positions: Default::default(),
            dfs: Some(Default::default()),
            cycles: Some(Default::default()),
            closure_walk: Some(Default::default()),
            distinct: Some(Default::default()),
        }
    }
}

/// The engine constructs and releases this borrow; a primitive owns no service state.
/// Borrowed logical work counters; storage belongs to the invocation owner.
#[derive(Default)]
pub(crate) struct WorkCounters([std::cell::Cell<u64>; 3]);
impl WorkCounters {
    pub(crate) fn found(&self, input: usize, rows: u64) {
        if let Some(total) = self.0.get(input) {
            total.set(total.get().saturating_add(rows));
        }
    }
    pub(crate) fn take(&self) -> [u64; 3] {
        std::array::from_fn(|at| self.0[at].replace(0))
    }
    pub(crate) fn snapshot(&self) -> [u64; 3] {
        std::array::from_fn(|at| self.0[at].get())
    }
}
pub(crate) struct PrimitiveLease<'a> {
    pub views: PrimitiveViews<'a>,
    pub workspace: &'a mut PrimitiveWorkspace,
    pub derived: QuerySymbols<'a>,
    predicate: &'a mut dyn PredicateProgram,
    semantics: &'a dyn PrimitiveSemantics,
    work: &'a WorkCounters,
}

/// Borrowed portable primitive meanings; the application supplies the implementation.
/// No outbound adapter, cache owner or scheduler is reachable from a primitive.
pub(crate) trait PrimitiveSemantics: Sync {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> super::views::ProgressObserver {
        |_, _| {}
    }

    fn operators(
        &self,
        call: ryo_operators::domain::contract::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >;
}

pub(crate) fn portable_fault(
    error: ryo_operators::domain::contract::types::PortableOperatorFault,
) -> OperatingError {
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    match error {
        PortableOperatorFault::Interrupted => OperatingError::WrongOperation,
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
#[derive(Default)]
pub(crate) struct DistinctWorkspace {
    pub rows: ryo_values::domain::rows::types::Table,
    pub seen: hashbrown::HashTable<ryo_values::domain::rows::types::RowIndex>,
}

/// Prepared metadata belongs to the request; workers only borrow this immutable value.
#[derive(Debug)]
pub(crate) struct RequestOperators {
    /// Original-plan shape metadata owned by this exact immutable request, never file state.
    pub ordered_existence: std::sync::Arc<Vec<crate::domain::ordered_existence::types::Program>>,
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
    #[cfg(feature = "diagnostic-row-progress")]
    pub(crate) fn progress(
        &self,
        counter: crate::domain::operator::contract::views::ProgressCounter,
        amount: u64,
    ) {
        (self.semantics.progress())(counter, amount);
    }

    /// Copy the request-lived semantic port before borrowing this lease for row delivery.
    pub(crate) fn semantics(&self) -> &'a dyn PrimitiveSemantics {
        self.semantics
    }

    pub(super) fn new(
        views: PrimitiveViews<'a>,
        workspace: &'a mut PrimitiveWorkspace,
        derived: QuerySymbols<'a>,
        predicate: &'a mut dyn PredicateProgram,
        semantics: &'a dyn PrimitiveSemantics,
        work: &'a WorkCounters,
    ) -> Self {
        Self {
            views,
            workspace,
            derived,
            predicate,
            semantics,
            work,
        }
    }
    pub(crate) fn found(&self, input: usize, rows: u64) {
        self.work.found(input, rows);
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

/// An engine error takes precedence even if an injected kernel returns a dishonest answer.
pub(crate) fn join_result(
    answer: Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >,
    error: Option<OperatingError>,
) -> Result<(), OperatingError> {
    written_result(answer, error)
}

/// Preserve the engine's original downstream error ahead of a portable interruption marker.
pub(crate) fn written_result(
    answer: Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >,
    error: Option<OperatingError>,
) -> Result<(), OperatingError> {
    if let Some(error) = error {
        return Err(error);
    }
    match answer {
        Ok(ryo_operators::domain::contract::types::OperatorAnswer::Written) => Ok(()),
        Ok(_) => Err(OperatingError::WrongOperation),
        Err(error) => Err(portable_fault(error)),
    }
}
