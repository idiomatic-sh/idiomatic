//! Borrowed equality join contracts; no engine authority.
use super::errors::EqualityJoinError;

/// Finite borrowed access to already acquired matching rows, in their original bag order.
/// Implementations cannot be selected or retained by the semantic kernel. One object
/// owns the exclusive borrow used for admission and downstream delivery.
pub trait JoinDriver {
    /// Advance to the next admitted match for this left key. Every yielded candidate
    /// must satisfy the prepared equality or native lookup contract; the kernel does
    /// not repeat verified key comparisons. Empty sources never access output cells.
    ///
    /// # Errors
    ///
    /// [`EqualityJoinError::Interrupted`] when the engine interrupts the join.
    fn next(
        &mut self,
        key: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<Advance, EqualityJoinError>;
    /// Decode one current right cell; no full-width row is required.
    fn right_cell(
        &self,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
    /// Deliver one projected row. After interruption every later call must remain interrupted.
    ///
    /// # Errors
    ///
    /// [`EqualityJoinError::Interrupted`] when the engine interrupts the join.
    fn emit(
        &mut self,
        row: &[ryo_relations::domain::relations::types::Value],
    ) -> Result<(), EqualityJoinError>;
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
        left: PositionSlots<'a>,
        /// One finite ordered source of already matching position rows.
        driver: &'a mut dyn PositionJoinDriver,
        /// Caller-owned flat position destination; publication remains engine-owned.
        target: PositionTarget<'a>,
    },
}
/// Borrowed matching position rows. No semantic cell is decoded by this interface.
pub trait PositionJoinDriver {
    /// Advance in the original bag order. The first error is sticky.
    ///
    /// # Errors
    ///
    /// [`EqualityJoinError::Interrupted`] when the engine interrupts the join.
    fn next(&mut self) -> Result<Advance, EqualityJoinError>;
    /// File-local position slots of the current already matching RHS row.
    fn right_positions(&self) -> PositionSlots<'_>;
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

/// The file-local position slots of the scheduled left row.
#[derive(Clone, Copy, Debug)]
pub struct PositionSlots<'a>(pub &'a [u32]);

/// The engine-owned flat position destination.
pub struct PositionTarget<'a>(pub &'a mut Vec<u32>);

/// Whether a driver advanced to another candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Advance {
    /// A candidate is current.
    Admitted,
    /// No candidate remains.
    Exhausted,
}

impl From<bool> for Advance {
    /// An advance that found a candidate is `Admitted`.
    fn from(found: bool) -> Self {
        if found {
            Self::Admitted
        } else {
            Self::Exhausted
        }
    }
}

/// What a join reads from its operation: its key columns, its output, the declared right view, and the
/// reachability lookup of a logical closure join.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JoinShape<'a> {
    /// The left key columns.
    pub left_on: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
    /// The right key columns.
    pub right_on: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
    /// The output columns.
    pub output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
    /// The logical-to-physical right column remapping.
    pub view: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
    /// The reachability lookup of a join over a closure.
    pub reach: Option<&'a ryo_values::domain::plan_parts::types::ReachLookup>,
}

/// A complete reachability answer the engine already holds for one request: the partition keys and the
/// source, then each destination in typed sort order.
#[derive(Clone, Copy, Debug)]
pub struct CachedReach<'a> {
    /// The partition keys followed by the source.
    pub request: &'a [ryo_relations::domain::relations::types::Value],
    /// The reached destinations.
    pub destinations: &'a [ryo_relations::domain::relations::types::Value],
}

/// An engine driver that lends its join candidates, and its closure walk for a reaching join.
pub trait JoinEvaluation: crate::domain::closure::types::ClosureEvaluation {
    /// The engine's candidate source of an equality join: an index probe or native tree candidates.
    fn join(&mut self) -> &mut dyn JoinDriver;
}
