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

/// Finite access to acquired typed membership, invocation distinct state and delivery.
/// The semantic capability cannot select an index, retain a row or publish independently.
pub trait SetRowDriver {
    /// Whether the acquired right population has no rows.
    fn right_empty(&mut self) -> Result<Population, SetRowsError>;
    /// Test a full row or projected left key against the acquired right index.
    fn contains_right(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<Membership, SetRowsError>;
    /// Record an invocation-scoped first occurrence of a full row.
    fn first_time(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<Occurrence, SetRowsError>;
    /// Deliver one retained row in original encounter order.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), SetRowsError>;
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
    fn next(&mut self) -> Result<Advance, SetRowsError>;
    /// Whether the acquired right population has no rows.
    fn right_empty(&mut self) -> Result<Population, SetRowsError>;
    /// Probe the current left row's key only when semantic policy asks.
    fn contains_right(&mut self) -> Result<Membership, SetRowsError>;
    /// Whether the current row is the first occurrence in this invocation.
    fn first_time(&mut self) -> Result<Occurrence, SetRowsError>;
    /// Deliver current positions without decoding unused cells.
    fn emit(&mut self) -> Result<(), SetRowsError>;
}

use crate::domain::equality_join::types::Advance;

/// Whether the acquired right population has rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Population {
    /// The right population has no rows.
    Empty,
    /// The right population has at least one row.
    Populated,
}

impl From<bool> for Population {
    /// A right population reported empty is `Empty`.
    fn from(empty: bool) -> Self {
        if empty { Self::Empty } else { Self::Populated }
    }
}

/// Whether a probed key is in the acquired right index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Membership {
    /// The key is in the index.
    Present,
    /// The key is not in the index.
    Absent,
}

impl From<bool> for Membership {
    /// A key reported contained is `Present`.
    fn from(contained: bool) -> Self {
        if contained { Self::Present } else { Self::Absent }
    }
}

/// Whether a row or value was seen before in this invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occurrence {
    /// This is the first time it is seen.
    First,
    /// It was seen before.
    Repeat,
}

impl From<bool> for Occurrence {
    /// A row reported first-time is `First`.
    fn from(first: bool) -> Self {
        if first { Self::First } else { Self::Repeat }
    }
}
