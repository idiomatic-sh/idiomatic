//! Portable semantic declarations interpreted by the engine.

/// The source length used to validate a composed byte interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceByteLength(pub usize);

/// One portable semantic fault vocabulary for every borrowed operator call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortableOperatorFault {
    /// The engine stopped candidate acquisition or downstream delivery.
    Interrupted,
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

/// The result of one pure operator meaning, before engine publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorAnswer {
    /// Caller-owned scratch was filled.
    Written,
    /// Checked half-open span endpoints.
    SpanBounds([ryo_relations::domain::relations::types::Value; 2]),
    /// A composed half-open span, including a declared absent span.
    SpanBetween(ryo_relations::domain::relations::types::Value),
}

/// A borrowed request for one pure operator meaning. Storage and publication stay with the engine.
pub enum OperatorCall<'a> {
    /// Apply one set or antijoin row meaning over engine-owned indexes and scratch.
    SetRow {
        /// Borrowed row and the already acquired physical membership boundary.
        input: SetInput<'a>,
    },
    /// Join one left row with the engine's ordered, already matching candidates.
    EqualityJoin {
        /// Borrowed values or deferred file-local position slots, selected by the engine.
        input: JoinInput<'a>,
    },
    /// Copy selected columns in plan order, preserving duplicates.
    Project {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Ordered column indices.
        columns: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Append a resolved row suffix without acquiring or publishing it.
    Append {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Resolved suffix.
        appended: &'a [ryo_relations::domain::relations::types::Value],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Read checked endpoints of one span cell.
    SpanBounds {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Declared span column.
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// Compose the start of one span with the end of another.
    SpanBetween {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Left span column.
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Right span column.
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Complete source byte length validated by the engine.
        source_len: SourceByteLength,
    },
}

/// The four set and antijoin meanings; input acquisition and delivery stay with the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetKind {
    /// Emit each distinct row from either side in encounter order.
    Union,
    /// Emit each distinct left row present on the right.
    Intersection,
    /// Emit each distinct left row absent on the right.
    Difference,
    /// Emit each left row whose projected key is absent on the right, retaining duplicates.
    Antijoin,
}

/// The original engine error remains with the finite driver after interruption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetInterrupted;

/// Finite access to acquired typed membership, invocation distinct state and delivery.
/// The semantic capability cannot select an index, retain a row or publish independently.
pub trait SetRowDriver {
    /// Whether the acquired right population has no rows.
    fn right_empty(&mut self) -> Result<bool, SetInterrupted>;
    /// Test a full row or projected left key against the acquired right index.
    fn contains_right(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<bool, SetInterrupted>;
    /// Record an invocation-scoped first occurrence of a full row.
    fn first_time(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<bool, SetInterrupted>;
    /// Deliver one retained row in original encounter order.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), SetInterrupted>;
}

/// Borrowed set input: typed rows or deferred tree positions selected by the engine.
pub enum SetInput<'a> {
    /// One scheduled typed row; `left_on` is consulted only for antijoin after a nonempty RHS.
    Values {
        /// Set or antijoin meaning.
        kind: SetKind,
        /// Scheduled left row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Ordered antijoin key columns.
        left_on: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Engine-owned finite membership, distinct and delivery boundary.
        driver: &'a mut dyn SetRowDriver,
        /// Caller-owned projected key scratch.
        key: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Deferred position rows over an engine-acquired raw key order.
    Positions {
        /// Set or antijoin meaning.
        kind: SetKind,
        /// Engine-owned finite position and delivery boundary.
        driver: &'a mut dyn SetPositionsDriver,
    },
}

/// Finite deferred rows; the engine owns raw ordering, decoding and output slots.
pub trait SetPositionsDriver {
    /// Advance to the next candidate in encounter order.
    fn next(&mut self) -> Result<bool, SetInterrupted>;
    /// Whether the acquired right population has no rows.
    fn right_empty(&mut self) -> Result<bool, SetInterrupted>;
    /// Probe the current left row's key only when semantic policy asks.
    fn contains_right(&mut self) -> Result<bool, SetInterrupted>;
    /// Whether the current row is the first occurrence in this invocation.
    fn first_time(&mut self) -> Result<bool, SetInterrupted>;
    /// Deliver current positions without decoding unused cells.
    fn emit(&mut self) -> Result<(), SetInterrupted>;
}

/// An interruption is retained as the engine's original error outside the capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JoinInterrupted;

/// Finite borrowed access to already acquired matching rows, in their original bag order.
/// Implementations cannot be selected or retained by the semantic kernel. One object
/// owns the exclusive borrow used for admission and downstream delivery.
pub trait JoinDriver {
    /// Advance to the next admitted match for this left key. Every yielded candidate
    /// must satisfy the prepared equality or native lookup contract; the kernel does
    /// not repeat verified key comparisons. Empty sources never access output cells.
    fn next(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<bool, JoinInterrupted>;
    /// Decode one current right cell; no full-width row is required.
    fn right_cell(
        &self,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
    /// Deliver one projected row. After interruption every later call must remain interrupted.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), JoinInterrupted>;
}

/// Ordinary Join representations. Dispatch is exhaustive at the single capability boundary.
pub enum JoinInput<'a> {
    /// Immediate typed cells over already acquired matching candidates.
    Values {
        /// The scheduled left row.
        left: JoinRow<'a>,
        /// Ordered left key columns, validated when this row is reached.
        left_on: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Ordered output projection, including duplicates.
        output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
        /// Logical-to-physical right column remapping; absent entries stay unchanged.
        view: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// One finite borrowed candidate source and delivery boundary.
        driver: &'a mut dyn JoinDriver,
        /// Caller-owned key buffer.
        key: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
        /// Caller-owned projected row buffer.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Deferred tree rows, concatenated without decoding their logical cells.
    Positions {
        /// File-local positions belonging to the scheduled left row.
        left: &'a [u32],
        /// One finite ordered source of already matching position rows.
        driver: &'a mut dyn PositionJoinDriver,
        /// Caller-owned flat position destination; publication remains engine-owned.
        target: &'a mut Vec<u32>,
    },
}
/// Borrowed matching position rows. No semantic cell is decoded by this interface.
pub trait PositionJoinDriver {
    /// Advance in the original bag order. The first error is sticky.
    fn next(&mut self) -> Result<bool, JoinInterrupted>;
    /// File-local position slots of the current already matching RHS row.
    fn right_positions(&self) -> &[u32];
}

/// One immutable left row: values already at hand, or a finite columnar reader.
pub enum JoinRow<'a> {
    /// Already materialized typed values.
    Values(&'a [ryo_relations::domain::relations::types::Value]),
    /// Decode only requested key/output cells from one already admitted row.
    Columns(&'a dyn JoinCells),
}
/// Read-only access to one already admitted physical row, never an acquisition port.
pub trait JoinCells {
    /// Read exactly one logical cell. No cache, scheduler or population handle is exposed.
    fn cell(
        &self,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
}
