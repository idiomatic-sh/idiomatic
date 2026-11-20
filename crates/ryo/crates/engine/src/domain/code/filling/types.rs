//! The types of the `filling` part of the `code` concept: the operands of the words of a pipeline's stages, each as the
//! encoding writes them and the executor reads them. A field is a slot of the word under its own name and type.

use crate::domain::code::types::{IndexSite, MemoSite, PoolAt, Short, WordFlag, WordSlot};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_language::domain::node_table::types::FieldCode;

/// The operands of `StageScan`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScanWord {
    /// The source under `SOURCE`; `BUCKET`; `OWN_TREE`.
    pub(crate) flag: WordFlag,
    /// The batch column each row's handle is written to.
    pub(crate) out: Register,
    /// The list of the bucket's constants, under `BUCKET`.
    pub(crate) constants: PoolAt,
    /// The relation's or the table's number; under `BUCKET` the index site of the source by the bucket's columns.
    pub(crate) read: WordSlot,
}

/// The operands of `StageProbe`: a pipeline's `Probe`, or a `Use` of a pattern in table form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProbeWord {
    /// The mode under `MODE`; `PROBE_MANY`, `PROBE_RANGE`, `PROBE_LOW_INCLUSIVE` and `PROBE_HIGH_INCLUSIVE`.
    pub(crate) flag: WordFlag,
    /// How many values.
    pub(crate) values: Short,
    /// The batch column each row's handle is written to.
    pub(crate) out: Register,
    /// The batch column holding the one value, or under `PROBE_MANY` the pool offset of the operands, the low and the high operands after them under `PROBE_RANGE`.
    pub(crate) probed: WordSlot,
    /// The index of the source by the columns.
    pub(crate) index: IndexSite,
}

/// The operands of `StageProduct`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProductWord {
    /// The source under `SOURCE`.
    pub(crate) flag: WordFlag,
    /// The batch column each row's handle is written to.
    pub(crate) out: Register,
    /// The relation's or the table's number.
    pub(crate) read: WordSlot,
}

/// The operands of `StageUse`: a `Use` of a pattern in call form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UseWord {
    /// The mode under `MODE`.
    pub(crate) flag: WordFlag,
    /// How many output columns.
    pub(crate) outputs: Short,
    /// The body record.
    pub(crate) body: PoolAt,
    /// That many output registers.
    pub(crate) registers: PoolAt,
    /// The memo site of the pattern.
    pub(crate) memo: MemoSite,
}

/// The operands of `StageRead`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StageReadWord {
    /// The source under `SOURCE`.
    pub(crate) flag: WordFlag,
    /// The column, as its source's column code.
    pub(crate) column: Short,
    /// The batch column holding the row's handle.
    pub(crate) row: Register,
    /// The batch column the cell is written to.
    pub(crate) out: Register,
    /// The relation's or the table's number.
    pub(crate) read: WordSlot,
}

/// The operands of `StageWalk`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WalkWord {
    /// `WALK_AXIS` for an axis, else the children; the direction under `WALK_DIRECTION`.
    pub(crate) flag: WordFlag,
    /// The field of a walk of one field's children; the code of no field otherwise.
    pub(crate) field: FieldCode,
    /// The batch column holding the start.
    pub(crate) start: Register,
    /// The batch column each element reached is written to.
    pub(crate) out: Register,
}

/// The operands of `StageWrite`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WriteWord {
    /// `REPEATS_DROPPED`.
    pub(crate) flag: WordFlag,
    /// How many columns the row has.
    pub(crate) columns: Short,
    /// The table written.
    pub(crate) table: MadeTableId,
    /// That many operands.
    pub(crate) operands: PoolAt,
}
