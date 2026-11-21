//! The types of the `bodies` part.

use crate::domain::code::types::{CodeEntry, MemoSite, PoolAt, WordSlot};

/// What a use of a pattern in call form names: its program, its memo site, its unit, and its bound columns' formals.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CallUse {
    /// Where the pattern's program starts.
    pub(crate) entry: CodeEntry,
    /// Where its answers are kept.
    pub(crate) site: MemoSite,
    /// Its unit among the code's.
    pub(crate) unit: WordSlot,
    /// How many columns are bound.
    pub(crate) bound: WordSlot,
    /// Where the pairs of a formal register and the caller's operand start.
    pub(crate) formals: PoolAt,
}
