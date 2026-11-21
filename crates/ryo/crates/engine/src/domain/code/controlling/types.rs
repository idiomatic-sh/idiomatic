//! The types of the `controlling` part of the `code` concept: the operands of the words over bodies and the words that end a program or hand a row over, each as the
//! encoding writes them and the executor reads them. A field is a slot of the word under its own name and type.

use crate::domain::code::types::{MemoSite, PoolAt, Short, WordFlag, WordSlot};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The operands of `Exists`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExistsWord {
    /// `ABSENT`; the memo's storage under `STORAGE`.
    pub(crate) flag: WordFlag,
    /// How many registers key the memo.
    pub(crate) keys: Short,
    /// The body record.
    pub(crate) body: PoolAt,
    /// That many key registers.
    pub(crate) key: PoolAt,
    /// The memo site.
    pub(crate) memo: MemoSite,
}

/// The operands of `Or`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrWord {
    /// The memo's storage under `STORAGE`.
    pub(crate) flag: WordFlag,
    /// How many bodies.
    pub(crate) bodies: Short,
    /// That many body records, one after the other.
    pub(crate) records: PoolAt,
    /// The list of the output registers, then the list of the registers that key the memo.
    pub(crate) outputs: PoolAt,
    /// The memo site.
    pub(crate) memo: MemoSite,
}

/// The operands of `Fork`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ForkWord {
    /// How many slots.
    pub(crate) slots: Short,
    /// The list of the slots' entries.
    pub(crate) entries: PoolAt,
}

/// The operands of `Yield`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct YieldWord {
    /// How many registers the row has.
    pub(crate) cells: Short,
    /// The register when the row has one, else the pool offset of the list of them.
    pub(crate) row: WordSlot,
}

/// The operands of `Emit`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct EmitWord {
    /// `FRONTIER` for a row of the rule's frontier entry.
    pub(crate) flag: WordFlag,
    /// How many columns the row has.
    pub(crate) columns: Short,
    /// The rule.
    pub(crate) rule: SelectedRuleOrdinal,
    /// The frontier entry's number, under `FRONTIER`.
    pub(crate) input: WordSlot,
    /// That many operands.
    pub(crate) operands: PoolAt,
}
