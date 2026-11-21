//! The shape every word of the `steps` collection implements, and what a word reads and writes: the file lent for the
//! scan, the worker's per-file state and the rule set's programs. An item is a leaf: it reads its word through the code's
//! readers, binds registers and pushes or moves a cursor. What runs a body, makes a table or builds an index is the loop's
//! (`executing`), which calls an item only when what the item reads is there.

use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::findings::types::FoundRows;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::{IterationCursor, LoopState};
use ryo_language::domain::facts::types::FileFacts;

/// What a word runs over: the lent file, the worker's per-file state, the programs it belongs to and where rows go.
pub(crate) struct StepFrame<'s, 'f> {
    /// The file: its node table, bytes and facts.
    pub(crate) file: &'s mut FileFacts<'f>,
    /// The registers, indexes, made tables, kept answers and stacks of the file.
    pub(crate) state: &'s mut LoopState,
    /// The rule set's programs.
    pub(crate) programs: &'s ProgramsEngine,
    /// Where found rows and notices go.
    pub(crate) found: &'s mut dyn FoundRows,
}

/// What a word answers the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flow {
    /// The next word runs.
    Next,
    /// This binding is done: the innermost running iteration binds its next row, or the program ends. A word that iterates
    /// answers it after pushing its cursor, so its first row is bound as every later one is.
    Back,
}

/// What every word of the collection offers. A word is one item; the loop dispatches a word's code to it.
pub(crate) trait InstructionsItem {
    /// Runs the word at `at` for the current candidate: a test answers whether the candidate goes on; an iteration pushes
    /// its cursor and answers `Back`.
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) -> Flow;

    /// Binds the next row of the word's running iteration, whose cursor is `cursor`, and answers where the loop goes on;
    /// nowhere when the iteration has no row left, or for a word that does not iterate.
    fn next(
        _frame: &mut StepFrame<'_, '_>,
        _word: CodeWord,
        _cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        None
    }
}
