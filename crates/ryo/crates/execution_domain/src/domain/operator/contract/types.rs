//! Prepared metadata is request-owned; invocation capabilities borrow engine-owned data.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::presence::{types::PresenceMask, types::RowFailure};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::Inputs;
use ryo_values::domain::plans::types::Operation;
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
    /// What the operation changes in symbolic state when it succeeds.
    pub effects: Effects,
    /// How widely the operation's result may be shared.
    pub reuse: ReuseScope,
    /// Exact typed fields, not a hash-only or Debug-text key.
    pub identity: Operation,
}

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
