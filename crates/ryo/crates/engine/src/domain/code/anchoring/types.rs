//! The types of the `anchoring` part of the `code` concept: the operands of the words that give the rows of a source: a kind's ids, the rows that hold values, the rows of a range, every row, and the rows of an ordered table nearest a bound, each as the
//! encoding writes them and the executor reads them. A field is a slot of the word under its own name and type.

use crate::domain::code::types::{IndexSite, PoolAt, RelationNumber, Short, WordFlag, WordSlot};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_relations::domain::relations::types::KindBits;

/// The operands of `EachKind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachKindWord {
    /// `SEVERAL` when more than one kind is iterated; `OWN_TREE`.
    pub(crate) flag: WordFlag,
    /// The kind, when one.
    pub(crate) kind: KindBits,
    /// The register each id is written to.
    pub(crate) out: Register,
    /// The list of the kinds, under `SEVERAL`.
    pub(crate) kinds: PoolAt,
}

/// The operands of `EachMatchNode` and `EachMatchFilled`: one column, one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachMatchWord {
    /// `CONSTANT` when the value is a constant; `OWN_TREE` over the node table.
    pub(crate) flag: WordFlag,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// The register holding the value, or a constant's number under `CONSTANT`.
    pub(crate) value: WordSlot,
    /// The index of the source by the column.
    pub(crate) index: IndexSite,
}

/// The operands of `EachMatchFilledMany`: several columns, as many values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachMatchManyWord {
    /// How many values.
    pub(crate) values: Short,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// That many operands.
    pub(crate) operands: PoolAt,
    /// The index of the relation by the columns.
    pub(crate) index: IndexSite,
}

/// The operands of `EachMatchMade`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachMatchMadeWord {
    /// `CONSTANT`, `MANY` and `WRITTEN`.
    pub(crate) flag: WordFlag,
    /// How many values, under `MANY`.
    pub(crate) values: Short,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// The one value (a register, or a constant's number under `CONSTANT`), or under `MANY` the pool offset of the operands.
    pub(crate) value: WordSlot,
    /// The index of the table by the columns.
    pub(crate) index: IndexSite,
}

/// The operands of `EachRange`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachRangeWord {
    /// `LOW_PRESENT`, `LOW_INCLUSIVE`, `HIGH_PRESENT`, `HIGH_INCLUSIVE`, `RANGE_OUT` and `OWN_TREE`.
    pub(crate) flag: WordFlag,
    /// How many match values.
    pub(crate) values: Short,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// That many operands, then the low operand, the high operand and the register the range column is written to.
    pub(crate) operands: PoolAt,
    /// The ordered index of the source by the match columns.
    pub(crate) index: IndexSite,
}

/// The operands of `EachRowNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachRowNodeWord {
    /// `OWN_TREE`.
    pub(crate) flag: WordFlag,
    /// The register each element's id is written to.
    pub(crate) out: Register,
}

/// The operands of `EachRowFilled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachRowFilledWord {
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// The relation.
    pub(crate) relation: RelationNumber,
}

/// The operands of `EachRowMade`: a made table's rows, a written table's, or a definition's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EachRowMadeWord {
    /// `WRITTEN` for a table that is the same for every file.
    pub(crate) flag: WordFlag,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// The table.
    pub(crate) table: MadeTableId,
}

/// The operands of `Latest` and `StageLatest`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LatestWord {
    /// The bound's side under `SIDE`; `BOUND_CONSTANT`.
    pub(crate) flag: WordFlag,
    /// How many key operands.
    pub(crate) keys: Short,
    /// The register each row's handle is written to.
    pub(crate) out: Register,
    /// The register holding the bound, or a constant's number under `BOUND_CONSTANT`.
    pub(crate) bound: WordSlot,
    /// The index site of the ordered table, then that many key operands.
    pub(crate) operands: PoolAt,
}
