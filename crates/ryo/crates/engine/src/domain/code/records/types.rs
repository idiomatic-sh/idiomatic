//! The types of the `records` part of the `code` concept: what the pool and the need formulas hold, as the encoding writes them
//! and the executor reads them.

use crate::domain::code::types::{
    CodeEntry, ConstantNumber, IndexSite, MemoSite, NeedSite, PoolAt, RelationNumber, WordSlot,
};
use crate::domain::steps::types::{MadeColumn, MadeTableId, Register};

/// An operand in the pool: two slots, a tag and a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PooledOperand {
    /// Tag 0: a register.
    Register(Register),
    /// Tag 1: a constant, by number.
    Constant(ConstantNumber),
    /// Tag 2: a sorted list of constants, by its place in the pool.
    Constants(PoolAt),
    /// Tag 3: the scanned file.
    File,
}

/// One bound column of a use of a pattern in call form: the formal the caller's cell arrives in, and the cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BoundFormal {
    /// The definition's formal of the bound column.
    pub(crate) formal: Register,
    /// The caller's value for it.
    pub(crate) value: PooledOperand,
}

/// One collected column of a use of a pattern in table form: the column read of a row found, and where it goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CollectedRead {
    /// The column of the definition's table.
    pub(crate) column: MadeColumn,
    /// The register it is written to; `NO_SLOT` where the step that runs the body takes the cell itself (a walk's next element).
    pub(crate) out: Register,
}

/// A body record: what a control word runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BodyRecord {
    /// `0, entry`: steps written in place.
    Inline {
        /// Where they start.
        entry: CodeEntry,
    },
    /// `1, entry, memo site, unit, n`, then n pairs of a formal register and an operand: a pattern in call form.
    Call {
        /// Where the pattern's program starts.
        entry: CodeEntry,
        /// Where its answers are kept, by the cells of its bound columns.
        memo: MemoSite,
        /// The pattern's unit: its place among the code's units.
        unit: WordSlot,
        /// How many columns are bound.
        bound: WordSlot,
        /// Where the pairs start.
        formals: PoolAt,
    },
    /// `2, index site, n`, then n operands; then `m` and m pairs of a column and an output register: a pattern in table form.
    Table {
        /// The index of the definition's table by the bound columns.
        index: IndexSite,
        /// How many columns are bound.
        bound: WordSlot,
        /// Where their operands start.
        values: PoolAt,
        /// How many columns are collected.
        collected: WordSlot,
        /// Where their pairs start.
        reads: PoolAt,
    },
}

/// A body record read from the pool, and where the pool goes on after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadBody {
    /// The record.
    pub(crate) record: BodyRecord,
    /// The place after its last slot.
    pub(crate) after: PoolAt,
}

/// A need formula's record: what must hold of a file for a unit to have a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeedRecord {
    /// `0`: nothing is known before the unit runs.
    Unknown,
    /// `1, pool list of kinds`: a node of one of the kinds.
    Kinds(PoolAt),
    /// `2, relation, column or none, constant`: a row of the relation, with the value in the column when one is given.
    Row {
        /// The relation.
        relation: RelationNumber,
        /// The column's place in its relation, or `NO_SLOT`.
        column: WordSlot,
        /// The constant's number, or `NO_SLOT`.
        constant: WordSlot,
    },
    /// `3, need site`: another unit's decision.
    Site(NeedSite),
    /// `4, table`: the table's own need; when that holds, the table is made and has a row.
    Table(MadeTableId),
    /// `5, n, offsets`: every part holds.
    All {
        /// How many parts.
        parts: WordSlot,
        /// Where their offsets start, each the place of a record among the needs.
        offsets: WordSlot,
    },
    /// `6, n, offsets`: some part holds.
    Any {
        /// How many parts.
        parts: WordSlot,
        /// Where their offsets start, each the place of a record among the needs.
        offsets: WordSlot,
    },
}

/// The columns of an edge row a graph fill's kernel is told: where an edge leaves from and where it arrives. What a `follow`
/// and a `cycles` fill hold, and the record the table's one parameter names: `source, destination`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EdgeColumns {
    /// The column of the entity an edge leaves.
    pub(crate) source: MadeColumn,
    /// The column of the entity it arrives at: a cell of the source's kind.
    pub(crate) destination: MadeColumn,
}
