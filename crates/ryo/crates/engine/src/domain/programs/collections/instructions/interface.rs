//! The shape every word of the `instructions` collection implements. An item is a leaf over a frame
//! (`programs::frames`): it reads its word through the code's readers, reads cells and rows through the seam, binds registers and pushes
//! or moves a cursor. What makes a table, fills a relation, builds an index or runs a body is the loop's (`executing`).

use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::scratch::types::IterationCursor;

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
