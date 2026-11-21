//! The types of the `executing` concept: what a program is run for.

use crate::domain::code::types::CodeEntry;
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::collections::instructions::interface::StepFrame;

/// What a program is run for, which says what its `Yield` and its end mean.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RunMode {
    /// A rule's branch: every row is emitted, and the program runs to its end.
    Rows,
    /// A body asked only whether it has a row: the first row ends it.
    Exists,
    /// A body or a pattern whose rows are collected: every `Yield` appends its row, and the program runs to its end.
    Collect,
}

/// The loop, as what asks a body hands it back: a body's part runs a program through the one loop without naming it.
pub(crate) type Runner = fn(&mut StepFrame<'_, '_>, CodeEntry, RunMode) -> InstructionCount;
