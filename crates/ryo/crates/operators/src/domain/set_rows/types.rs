//! Borrowed set rows contracts; no engine authority.
use super::errors::SetRowsError;
use crate::domain::contract::types::FirstOccurrence;
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
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn right_empty(&mut self) -> Result<RightPopulation, SetRowsError>;
    /// Test a full row or projected left key against the acquired right index.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn contains_right(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<RightMembership, SetRowsError>;
    /// Record an invocation-scoped first occurrence of a full row.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn first_time(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<FirstOccurrence, SetRowsError>;
    /// Deliver one retained row in original encounter order.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
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
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn next(&mut self) -> Result<Advance, SetRowsError>;
    /// Whether the acquired right population has no rows.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn right_empty(&mut self) -> Result<RightPopulation, SetRowsError>;
    /// Probe the current left row's key only when semantic policy asks.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn contains_right(&mut self) -> Result<RightMembership, SetRowsError>;
    /// Whether the current row is the first occurrence in this invocation.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn first_time(&mut self) -> Result<FirstOccurrence, SetRowsError>;
    /// Deliver current positions without decoding unused cells.
    ///
    /// # Errors
    ///
    /// [`SetRowsError::Interrupted`] when the engine interrupts the set operation.
    fn emit(&mut self) -> Result<(), SetRowsError>;
}

use crate::domain::equality_join::types::Advance;

/// Whether the acquired right population has rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightPopulation {
    /// The right population has no rows.
    Empty,
    /// The right population has at least one row.
    Populated,
}

impl From<bool> for RightPopulation {
    /// A right population reported empty is `Empty`.
    fn from(empty: bool) -> Self {
        if empty { Self::Empty } else { Self::Populated }
    }
}

/// Whether a probed key is in the acquired right index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightMembership {
    /// The key is in the index.
    Present,
    /// The key is not in the index.
    Absent,
}

impl From<bool> for RightMembership {
    /// A key reported contained is `Present`.
    fn from(contained: bool) -> Self {
        if contained {
            Self::Present
        } else {
            Self::Absent
        }
    }
}

/// An engine driver that lends its set-row boundary.
pub trait SetRowsEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's membership, distinct and delivery boundary of a set row.
    fn set_rows(&mut self) -> &mut dyn SetRowDriver;
}

/// What a set meaning lowers to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SetLowering {
    /// Each row of the left rows, then of the right, the first time only.
    Distinct,
    /// The left rows a probe of the right ones keeps.
    Probe(crate::domain::equality_join::types::ProbeParts),
}
