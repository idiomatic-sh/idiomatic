//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use super::views::{IndexedRows, PredicateProgram, ValidatedSource};
use crate::domain::table_model::types::RepresentationInput;
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
pub struct SemanticContract {
    pub shape: InputMode,
    pub(crate) demand: DemandOrder,
    pub acquisition: AcquisitionBoundary,
    pub row_inputs: RowInputMask,
    pub preflight: PreflightOrder,
}

/// What an operation declares of its semantics: its input contract, its effects and the
/// inputs whose absence of rows it trusts.
pub(crate) struct OperationSemantics {
    pub contract: SemanticContract,
    pub effects: Effects,
    pub absence: crate::domain::contributor_rows::types::AbsenceMask,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceLifetime {
    Invocation,
    PrimaryRow,
}
#[derive(Clone, Debug)]
pub enum WorkspaceProgram {
    None,
    FirstOccurrences {
        lifetime: WorkspaceLifetime,
        kinds: Result<Vec<CellKind>, OperatingError>,
    },
}

/// Owned by one request and aligned with the compiled operation.
#[derive(Clone, Debug)]
pub struct OperatorContract {
    pub derived: crate::domain::derived_views::types::contract::value::DerivedSlots,
    pub kind: OperationKind,
    pub inputs: Inputs,
    pub input_kinds: Vec<Vec<CellKind>>,
    pub semantics: SemanticContract,
    /// Metadata errors remain deferred to the declared execution boundary.
    pub schema: Result<Vec<CellKind>, OperatingError>,
    /// Payload-independent collected layout; row-lazy refusals are not raised by it.
    pub output_kinds: Result<Vec<CellKind>, OperatingError>,
    /// Input-position bitset: absence-dependent outputs require these populations complete.
    pub absence_inputs: crate::domain::contributor_rows::types::AbsenceMask,
    pub workspace: WorkspaceProgram,
    pub resources: super::resources::ResourceProgram,
    pub empty_results: Vec<EmptyResult>,
    pub source_gate: SourceGate,
    pub row_error: Option<OperatingError>,
    pub literal_pattern: Option<ryo_values::domain::plan_parts::types::LiteralText>,
    pub predicates: PreparedPredicates,
    pub(crate) effects: Effects,
    pub(crate) reuse: ReuseScope,
    /// Exact typed fields, not a hash-only or Debug-text key.
    pub identity: Operation,
}

/// Immutable inputs only. No Feed, callback, scheduler or owned/retained Arc is supplied.
#[derive(Clone, Copy)]
pub enum PreparedInputs<'a> {
    Row {
        input: InputPlace,
        row: &'a [Value],
        sides: &'a [RepresentationInput<'a>],
    },
    Views(&'a [RepresentationInput<'a>]),
    RequestSource(&'a RepresentationInput<'a>),
}

/// Read-only data borrowed from the application-owned invocation. No owner,
/// scheduler, acquisition or release operation crosses this interface.
pub trait PrimitiveResources {
    fn validate_declared(&self) -> Result<(), OperatingError>;
    fn index(&self) -> Option<IndexedRows<'_>>;
    fn appended(&self) -> Option<&[Value]>;
    fn rank_layers(&self) -> Option<&[Vec<ryo_values::domain::plan_parts::types::LiteralText>]>;
    fn axis(&self) -> Result<Option<super::views::PreparedAxis<'_>>, OperatingError>;
    fn native_join(&self) -> Option<super::views::NativeJoin<'_>>;
    fn dfs(
        &self,
    ) -> Result<Option<&ryo_derived::domain::dfs::types::PreparedDfs>, OperatingError>;
    fn cycles(
        &self,
    ) -> Result<Option<&ryo_derived::domain::cycle_adjacency::types::CycleGraph>, OperatingError>;
    fn closure(&self) -> Result<super::views::ClosureViews<'_>, OperatingError>;
    fn ordered(
        &self,
    ) -> Result<Option<&ryo_derived::domain::ordered::types::OrderedMatching>, OperatingError>;
    fn groups(&self) -> Option<&[crate::domain::table_groups::types::Group]>;
}

/// Only row-varying context is assembled at row delivery. Acquired resources
/// remain in the invocation and are reached through a borrowed reader.
pub struct PrimitiveViews<'a> {
    pub phase: crate::domain::operating::types::AfterTheFiles,
    pub resources: &'a dyn PrimitiveResources,
    pub semantic: Option<&'a dyn crate::domain::operating::types::SemanticLookup>,
    pub paths: &'a [ryo_language::domain::files::types::FilePath],
    pub arenas: &'a [ryo_values::domain::stored_facts::types::ArenaBytes],
    pub source: SourceView<'a>,
    pub bindings: &'a [ParameterBinding],
    pub validated_source: Option<ValidatedSource<'a>>,
    pub regex: Option<&'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    pub cached_reach: Option<super::graph_view::ReachResult<'a>>,
    pub cached_axis: Option<super::graph_view::AxisResult<'a>>,
}

/// Allocation slots owned once by the application invocation. A primitive only
/// receives a reborrow; no cache, owner or generation identity is reachable.
pub struct PrimitiveWorkspace {
    /// The buffers the operators fill while they evaluate a kind.
    pub evaluation: ryo_operators::domain::evaluation::types::EvaluationScratch,
    pub(crate) axis: Option<AxisWorkspace>,
    pub(crate) positions: Vec<ryo_relations::domain::relations::types::Position>,
    pub(crate) dfs: Option<DfsWorkspace>,
    pub(crate) closure_walk: Option<crate::domain::table_reach::types::Walk>,
    pub(crate) distinct: Option<DistinctWorkspace>,
}
/// The rows found at each of the three inputs of a counted step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkTotals(pub [FoundRows; 3]);

/// How many rows a counted step found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FoundRows(pub u64);

/// The engine constructs and releases this borrow; a primitive owns no service state.
/// Borrowed logical work counters; storage belongs to the invocation owner.
#[derive(Default)]
pub struct WorkCounters(pub(super) [std::cell::Cell<u64>; 3]);
pub struct PrimitiveLease<'a> {
    pub(crate) views: PrimitiveViews<'a>,
    pub(crate) workspace: &'a mut PrimitiveWorkspace,
    pub derived: QuerySymbols<'a>,
    pub(super) predicate: &'a mut dyn PredicateProgram,
    pub(super) semantics: &'a dyn PrimitiveSemantics,
    pub(super) work: &'a WorkCounters,
}

/// Borrowed portable primitive meanings; the application supplies the implementation.
/// No outbound adapter, cache owner or scheduler is reachable from a primitive.
pub trait PrimitiveSemantics: Sync {
    #[cfg(feature = "diagnostic-row-progress")]
    fn progress(&self) -> &dyn super::views::ProgressObserver {
        &super::views::IgnoredProgress
    }

    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    >;
}

pub fn portable_fault(
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
        PortableOperatorFault::WrongBinding => OperatingError::WrongBinding,
        PortableOperatorFault::Unbound => OperatingError::Unbound,
        PortableOperatorFault::NotRun => OperatingError::NotRun,
        PortableOperatorFault::MissingCapture
        | PortableOperatorFault::WrongOperation
        | PortableOperatorFault::UnsupportedRender => OperatingError::WrongOperation,
    }
}

/// A narrow lease reborrow preserves source/derived symbols while downstream rows run.
pub trait RowSink {
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
pub struct PreparationCount(pub usize);

/// Prepared metadata belongs to the request; workers only borrow this immutable value.
#[derive(Debug)]
pub struct RequestOperators {
    /// Original-plan shape metadata owned by this exact immutable request, never file state:
    /// one slot per rule, compiled when the request graph first resolves that rule's regions.
    pub ordered_existence:
        Vec<std::sync::OnceLock<crate::domain::self_join_recogniser::types::Program>>,
    pub rules: Vec<Vec<std::sync::Arc<OperatorContract>>>,
    pub preparations: PreparationCount,
}

/// Exact semantic program bound once; a primitive cannot substitute comparisons.
#[derive(Clone, Debug)]
pub struct PreparedPredicates {
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
pub enum EmptyRepresentation {
    Any,
    Computed,
}
#[derive(Clone, Debug)]
pub struct EmptyResult {
    pub input: ryo_values::domain::plan_parts::types::InputNumber,
    pub representation: EmptyRepresentation,
    pub kinds: Result<Vec<CellKind>, OperatingError>,
}

/// The work of one invocation of an axis walk.
#[derive(Debug, Default)]
pub struct AxisWorkspace {
    pub(crate) walk: crate::domain::table_reach::types::Walk,
    pub(crate) found: Vec<ryo_relations::domain::relations::types::Position>,
}

/// The vertices one invocation of the ordered depth-first search has expanded.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ExpandedCount(pub(crate) usize);

/// The edges one invocation of the ordered depth-first search has examined.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ExaminedCount(pub(crate) usize);

/// The work of one invocation of the ordered depth-first search, with what its walks expanded and examined.
#[derive(Debug, Default)]
pub struct DfsWorkspace {
    pub(crate) walk: crate::domain::table_reach::types::OrderedWalk,
    pub(crate) expanded: ExpandedCount,
    pub(crate) examined: ExaminedCount,
}
