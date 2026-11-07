//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{BorrowedRows, IndexedRows, PredicateProgram};
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::InputPlace;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::query_bindings::types::ParameterBinding;

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

/// One prepared semantic dependency and input contract for every output consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SemanticContract {
    pub(crate) shape: InputShape,
    pub(crate) demand: DemandOrder,
    pub(crate) acquisition: AcquisitionBoundary,
    pub(crate) row_inputs: RowInputMask,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceLifetime {
    Invocation,
    PrimaryRow,
}
#[derive(Clone, Debug)]
pub(crate) enum WorkspaceProgram {
    None,
    FirstOccurrences {
        lifetime: WorkspaceLifetime,
        kinds: Result<Vec<CellKind>, OperatingError>,
    },
}

/// Owned by one request and aligned with the compiled operation.
#[derive(Clone, Debug)]
pub(crate) struct OperatorContract {
    pub derived: crate::domain::derived_views::types::DerivedSlots,
    pub kind: OperationKind,
    pub inputs: Inputs,
    pub input_kinds: Vec<Vec<CellKind>>,
    pub semantics: SemanticContract,
    /// Metadata errors remain deferred to the declared execution boundary.
    pub schema: Result<Vec<CellKind>, OperatingError>,
    /// Payload-independent collected layout; row-lazy refusals are not raised by it.
    pub output_kinds: Result<Vec<CellKind>, OperatingError>,
    /// Input-position bitset: absence-dependent outputs require these populations complete.
    pub absence_inputs: u8,
    pub workspace: WorkspaceProgram,
    pub resources: super::resources::ResourceProgram,
    pub empty_results: Vec<EmptyResult>,
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
    fn closure(&self) -> Result<super::views::ClosureViews<'_>, OperatingError>;
    fn ordered(
        &self,
    ) -> Result<Option<&ryo_derived::domain::contract::types::OrderedMatching>, OperatingError>;
    fn groups(&self) -> Option<&[crate::domain::table_groups::types::Group]>;
}

/// Only row-varying context is assembled at row delivery. Acquired resources
/// remain in the invocation and are reached through a borrowed reader.
pub(crate) struct PrimitiveViews<'a> {
    pub phase: crate::domain::operating::types::AfterTheFiles,
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
    pub regex: Option<&'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    pub cached_reach: Option<super::graph_view::ReachResult<'a>>,
    pub cached_axis: Option<super::graph_view::AxisResult<'a>>,
}

/// The text an operator builds in place, handed back with its capacity kept.
#[derive(Debug, Default)]
pub(crate) struct ScratchText(pub String);

/// Allocation slots owned once by the application invocation. A primitive only
/// receives a reborrow; no cache, owner or generation identity is reachable.
pub(crate) struct PrimitiveWorkspace {
    pub row: Vec<Value>,
    pub other: Vec<Value>,
    pub text: ScratchText,
    pub word: crate::domain::table_scratch::types::ScratchWord,
    pub stretches: crate::domain::table_scratch::types::Stretches,
    pub captured: Vec<Value>,
    pub axis: Option<super::axis::AxisWorkspace>,
    pub positions: Vec<ryo_relations::domain::relations::types::Position>,
    pub dfs: Option<super::graph::DfsWorkspace>,
    pub cycles: Option<ryo_operators::domain::cycles::types::CycleScratch>,
    pub closure_walk: Option<crate::domain::table_reach::types::Walk>,
    pub distinct: Option<DistinctWorkspace>,
}
/// The engine constructs and releases this borrow; a primitive owns no service state.
/// Borrowed logical work counters; storage belongs to the invocation owner.
#[derive(Default)]
pub(crate) struct WorkCounters(pub(super) [std::cell::Cell<u64>; 3]);
pub(crate) struct PrimitiveLease<'a> {
    pub views: PrimitiveViews<'a>,
    pub workspace: &'a mut PrimitiveWorkspace,
    pub derived: QuerySymbols<'a>,
    pub(super) predicate: &'a mut dyn PredicateProgram,
    pub(super) semantics: &'a dyn PrimitiveSemantics,
    pub(super) work: &'a WorkCounters,
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
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
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
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: index.0 }.into()
        }
        PortableOperatorFault::ShapeMismatch => OperatingError::ShapeMismatch,
        PortableOperatorFault::InvalidSpanBounds => OperatingError::InvalidSpanBounds,
        PortableOperatorFault::InvalidSpan => OperatingError::InvalidSpan,
        PortableOperatorFault::InvalidSourceMatchSpan => OperatingError::InvalidSourceMatchSpan,
        PortableOperatorFault::UnboundKey => OperatingError::WrongBinding,
        PortableOperatorFault::MissingCapture
        | PortableOperatorFault::WrongOperation
        | PortableOperatorFault::UnsupportedRender => OperatingError::WrongOperation,
    }
}

/// A narrow lease reborrow preserves source/derived symbols while downstream rows run.
pub(crate) trait RowSink {
    fn row(&mut self, lease: &mut PrimitiveLease<'_>, row: &[Value]) -> Result<(), OperatingError>;
}

/// First-occurrence row semantics, bounded and released by the engine invocation.
pub(crate) struct DistinctWorkspace {
    pub schema: Result<(), OperatingError>,
    pub rows: ryo_values::domain::rows::types::Table,
    pub seen: hashbrown::HashTable<ryo_values::domain::rows::types::RowIndex>,
}

/// How many operations a request prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreparationCount(pub usize);

/// Prepared metadata belongs to the request; workers only borrow this immutable value.
#[derive(Debug)]
pub(crate) struct RequestOperators {
    /// Original-plan shape metadata owned by this exact immutable request, never file state.
    pub ordered_existence: std::sync::Arc<Vec<crate::domain::self_join_recogniser::types::Program>>,
    pub selection_plans: std::sync::Arc<Vec<crate::domain::stored_selection::types::PreparedPlan>>,
    pub selection_views:
        std::sync::Arc<crate::domain::derived_views::types::PreparedSelectionViews>,
    pub rules: Vec<Vec<std::sync::Arc<OperatorContract>>>,
    pub preparations: PreparationCount,
    pub physical: Vec<
        Vec<
            crate::domain::table_model::collections::representations::publication::PreparedPhysical,
        >,
    >,
}

/// Exact semantic program bound once; a primitive cannot substitute comparisons.
#[derive(Clone, Debug)]
pub(crate) struct PreparedPredicates {
    pub comparisons: Vec<ryo_values::domain::plan_parts::types::PlanComparison>,
    pub kinds: Vec<CellKind>,
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
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub representation: EmptyRepresentation,
    pub kinds: Result<Vec<CellKind>, OperatingError>,
}

/// An engine error takes precedence even if an injected kernel returns a dishonest answer.
pub(crate) fn join_result(
    answer: Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >,
    error: Option<OperatingError>,
) -> Result<(), OperatingError> {
    written_result(answer, error)
}

/// Preserve the engine's original downstream error ahead of a portable interruption marker.
pub(crate) fn written_result(
    answer: Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >,
    error: Option<OperatingError>,
) -> Result<(), OperatingError> {
    if let Some(error) = error {
        return Err(error);
    }
    match answer {
        Ok(ryo_operators::inbound::use_cases::operators::types::OperatorAnswer::Written) => {
            Ok(())
        }
        Ok(_) => Err(OperatingError::WrongOperation),
        Err(error) => Err(portable_fault(error)),
    }
}
