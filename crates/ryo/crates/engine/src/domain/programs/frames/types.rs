//! The types of the `frames` part: what a word runs over. A frame has two halves: the fill side's frame (the programs, the
//! file's node table and facts, its words, and the store of what was filled and made) and the loop's own state
//! (registers, kept answers, cursors), with where found rows go and the loop's entries.

use crate::domain::code::types::CodeEntry;
use crate::domain::findings::types::{FoundRows, InstructionCount};
use crate::domain::made_tables::types::{FillFrame, PatternAsk};
use crate::domain::memos::types::MemoId;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::scratch::types::LoopState;
use crate::domain::steps::types::MadeTableId;

/// What a word runs over.
pub(crate) struct StepFrame<'s, 'f> {
    /// The fill side's frame: the programs, the file, its words and the store.
    pub(crate) fill: &'s mut FillFrame<'f>,
    /// The loop's own per-file state.
    pub(crate) state: &'s mut LoopState,
    /// Where found rows and notices go.
    pub(crate) found: &'s mut dyn FoundRows,
    /// The loop, as what a part that runs a body or answers the fill side calls back.
    pub(crate) ports: &'s dyn LoopEntry,
}

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

/// The loop's three entries, as every part that would otherwise name the loop reaches them: a body's part runs a program,
/// and the answer to the fill side's callback asks a pattern or decides a table's need.
pub(crate) trait LoopEntry {
    /// Runs the program at `entry` for what `mode` says, and answers how many rows it gave.
    fn run(
        &self,
        frame: &mut StepFrame<'_, '_>,
        entry: CodeEntry,
        mode: RunMode,
    ) -> InstructionCount;

    /// The answer of a pattern in call form for a key, kept or evaluated.
    fn pattern(&self, frame: &mut StepFrame<'_, '_>, asked: PatternAsk<'_>) -> MemoId;

    /// Whether a made table may have a row in the file.
    fn table_need(&self, frame: &mut StepFrame<'_, '_>, table: MadeTableId) -> NeedVerdict;
}
