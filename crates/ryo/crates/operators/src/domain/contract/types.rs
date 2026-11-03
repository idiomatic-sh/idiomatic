//! Portable semantic declarations interpreted by the engine.

/// First input column missing from a pure row projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectColumnAbsent(pub u32);

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// Checked span semantics without an engine error or storage dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanFault {
    /// The declared input column is absent.
    NoSuchColumn {
        /// Missing column index.
        index: u32,
    },
    /// A present cell is not a span.
    ShapeMismatch,
    /// A span-bound extraction received an invalid interval.
    InvalidSpanBounds,
    /// A composed span exceeds the source or reverses its interval.
    InvalidSpan,
}

/// Portable semantic faults; the engine maps these to its existing public errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// Projection could not read the first absent column.
    Project(ProjectColumnAbsent),
    /// Checked span semantics rejected an operand.
    Span(SpanFault),
}

/// The result of one pure operator meaning, before engine publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorAnswer {
    /// Caller-owned scratch was filled.
    Written,
    /// Checked half-open span endpoints.
    SpanBounds([ryo_language::domain::relations::types::Value; 2]),
    /// A composed half-open span, including a declared absent span.
    SpanBetween(ryo_language::domain::relations::types::Value),
}

/// A borrowed request for one pure operator meaning. Storage and publication stay with the engine.
pub enum OperatorCall<'a> {
    /// Copy selected columns in plan order, preserving duplicates.
    Project {
        /// Input row.
        row: &'a [ryo_language::domain::relations::types::Value],
        /// Ordered column indices.
        columns: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_language::domain::relations::types::Value>,
    },
    /// Append a resolved row suffix without acquiring or publishing it.
    Append {
        /// Input row.
        row: &'a [ryo_language::domain::relations::types::Value],
        /// Resolved suffix.
        appended: &'a [ryo_language::domain::relations::types::Value],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_language::domain::relations::types::Value>,
    },
    /// Read checked endpoints of one span cell.
    SpanBounds {
        /// Input row.
        row: &'a [ryo_language::domain::relations::types::Value],
        /// Declared span column.
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// Compose the start of one span with the end of another.
    SpanBetween {
        /// Input row.
        row: &'a [ryo_language::domain::relations::types::Value],
        /// Left span column.
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Right span column.
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Complete source byte length validated by the engine.
        source_len: SourceByteLength,
    },
}

/// This affects error/publication timing, not which primitive implementation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluationMode {
    /// Emit rows as they are computed.
    Streaming,
    /// Consume a complete input before evaluating.
    Collected,
}

/// The finite existing semantic input shapes; dependency acquisition is engine work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputShape {
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
    /// Acquire join input at its declared transport boundary.
    JoinTransport,
    /// Acquire axis input at its declared transport boundary.
    AxisTransport,
    /// Skip right input when the left input proves empty.
    LeftThenRightUnlessEmptyLeft,
}

/// A complete-view algorithm may fail before any output is observable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    /// Emit successful rows incrementally.
    Incremental,
    /// Publish rows only after complete success.
    AfterSuccess,
}

/// Engine-owned lifetime of semantic first-occurrence work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetScope {
    /// No first-occurrence state is required.
    None,
    /// Share first-occurrence work for one invocation.
    Invocation,
    /// Restart first-occurrence work for each primary row.
    PrimaryRow,
}

/// Resource declarations are semantic needs, never primitive-controlled cache policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceNeeds {
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

/// Mode-specific failure and demand behavior is prepared once, never per row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeContract {
    /// Semantic form of inputs seen by the operator.
    pub shape: InputShape,
    /// Upstream demand order.
    pub demand: DemandOrder,
    /// Visibility boundary for ordinary evaluation.
    pub publication: Publication,
    /// Visibility boundary when input is held.
    pub held_publication: Publication,
    /// When schema validation is observable.
    pub schema_boundary: SchemaBoundary,
    /// When acquired resources are observable.
    pub acquisition: AcquisitionBoundary,
    /// Input positions read in row mode.
    pub row_inputs: RowInputMask,
    /// Whether the mode streams or collects.
    pub evaluation: EvaluationMode,
    /// Order of collected-mode preflight checks.
    pub preflight: PreflightOrder,
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

/// Collection validates the complete schema before reading; streaming row transforms
/// retain their first-applicable-row error boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaBoundary {
    /// Before upstream demand.
    BeforeDemand,
    /// At the first row that uses the schema.
    FirstApplicableRow,
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
