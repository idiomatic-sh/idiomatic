//! Borrowed set rows contracts; no engine authority.
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
