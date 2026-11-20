//! The types of the `deciding` part of the `code` concept: the operands of the words that read a cell, test, call a meaning and write a register, each as the
//! encoding writes them and the executor reads them. A field is a slot of the word under its own name and type.

use crate::domain::code::types::{
    ConstantNumber, PoolAt, RelationNumber, Short, WordFlag, WordSlot,
};
use crate::domain::steps::types::{MadeColumn, MadeTableId, RegexId, Register};

/// The operands of `ReadNode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadNodeWord {
    /// The node column's code.
    pub(crate) column: Short,
    /// The register holding the element.
    pub(crate) row: Register,
    /// The register the cell is written to.
    pub(crate) out: Register,
}

/// The operands of `ReadFilled`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadFilledWord {
    /// The column's place in its relation.
    pub(crate) place: Short,
    /// The register holding the row's handle.
    pub(crate) row: Register,
    /// The register the cell is written to.
    pub(crate) out: Register,
    /// The relation.
    pub(crate) relation: RelationNumber,
}

/// The operands of `ReadMade`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadMadeWord {
    /// The column.
    pub(crate) column: MadeColumn,
    /// The register holding the row's handle.
    pub(crate) row: Register,
    /// The register the cell is written to.
    pub(crate) out: Register,
    /// The table.
    pub(crate) table: MadeTableId,
}

/// The operands of `Test` and `StageKeep`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TestWord {
    /// `LEFT_CONSTANT`, `RIGHT_CONSTANT` and `RIGHT_SET`.
    pub(crate) flag: WordFlag,
    /// The comparison's code.
    pub(crate) comparison: Short,
    /// The left register, or a constant's number under `LEFT_CONSTANT`.
    pub(crate) left: WordSlot,
    /// The right register, a constant's number under `RIGHT_CONSTANT`, or the pool offset of a sorted list of constants under `RIGHT_SET`.
    pub(crate) right: WordSlot,
    /// The regular expression of a `Matches`.
    pub(crate) regex: RegexId,
}

/// The operands of `TestEqualConstant` and `TestNotEqualConstant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ConstantTestWord {
    /// The register tested.
    pub(crate) register: Register,
    /// The constant it is compared with.
    pub(crate) constant: ConstantNumber,
}

/// The operands of `TestEqualRegister` and `TestNotEqualRegister`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegisterTestWord {
    /// The left register.
    pub(crate) left: Register,
    /// The right register.
    pub(crate) right: Register,
}

/// The operands of `TestInConstants`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SetTestWord {
    /// `NEGATED` for a test that the set does not hold the value.
    pub(crate) flag: WordFlag,
    /// The register tested.
    pub(crate) register: Register,
    /// The sorted list of constants.
    pub(crate) constants: PoolAt,
}

/// The operands of `Call` and `StageCall`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CallWord {
    /// The meaning's code.
    pub(crate) meaning: Short,
    /// The list of the argument operands.
    pub(crate) arguments: PoolAt,
    /// The list of the output registers.
    pub(crate) outputs: PoolAt,
    /// The regular expression of a meaning that has one.
    pub(crate) regex: RegexId,
}

/// The operands of `Render` and `StageRender`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RenderWord {
    /// The context's code.
    pub(crate) context: Short,
    /// The register holding the node.
    pub(crate) node: Register,
    /// The register the rendered text is written to.
    pub(crate) out: Register,
}

/// The operands of `Let`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LetWord {
    /// `CONSTANT` or `THE_FILE`.
    pub(crate) flag: WordFlag,
    /// The register read, or a constant's number under `CONSTANT`.
    pub(crate) from: WordSlot,
    /// The register written.
    pub(crate) out: Register,
}
