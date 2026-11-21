//! `Call`: each row a per-row meaning gives the candidate, its cells bound to the word's outputs in turn; none ends the
//! candidate. The loop asks the meaning and stages the rows, and leaves where they are in the state; this item iterates
//! them and gives the room back when they are done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::deciding::service::call_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{listed, staged_cursor, staged_row_next};
use crate::domain::scratch::types::IterationCursor;

/// The `Call` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Call;

impl InstructionsItem for Call {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        staged_cursor(frame, at)
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        let code = frame.fill.programs.code;
        staged_row_next(frame, cursor, listed(&code.pool, call_of(word).outputs))
    }
}
