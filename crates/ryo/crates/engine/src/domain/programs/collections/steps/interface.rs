//! The shape every instruction of the `steps` collection implements: the operands the lowering writes for it, and how it runs for
//! the current candidate over the file, the scratch and the rule set's programs.
#![expect(
    dead_code,
    reason = "the collection interface the executor (track X) implements one item per instruction; until it lands no item exists, and its first item removes this"
)]

use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::ScratchEngine;
use ryo_language::domain::facts::types::FileFacts;

/// What an instruction runs over: the lent file, the worker's per-file state and the programs it belongs to.
pub(crate) struct StepFrame<'s, 'f> {
    /// The file: its node table, bytes and facts.
    pub(crate) file: &'s mut FileFacts<'f>,
    /// The registers, filled columns, made tables and words of the file.
    pub(crate) scratch: &'s mut ScratchEngine,
    /// The rule set's programs: shared sub-programs, made tables, constants.
    pub(crate) programs: &'s ProgramsEngine,
}

/// Whether a program goes on after a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    /// The next step runs.
    Continue,
    /// The candidate is done (a test failed, or an early exit was taken).
    Stop,
}

/// What every instruction offers. An instruction is one item; the executor dispatches `ProgramStep` to it.
pub(crate) trait StepsItem {
    /// The operands the lowering writes for it.
    type Operands;

    /// Runs the instruction for the current candidate: a filter answers once, an iterator calls `next` once per binding it
    /// writes (and answers `Stop` when `next` asks it to stop early).
    fn run(
        operands: &Self::Operands,
        frame: &mut StepFrame<'_, '_>,
        next: &mut dyn FnMut(&mut StepFrame<'_, '_>) -> Flow,
    ) -> Flow;
}
