//! `Or`: every row of its bodies, each bound to the word's outputs in turn. The loop stages the bodies' rows and leaves
//! where they are in the state; this item iterates them and gives the room back when they are done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::controlling::service::or_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{listed, staged_cursor, staged_row_next};
use crate::domain::scratch::types::IterationCursor;

/// The `Or` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Or;

impl InstructionsItem for Or {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        staged_cursor(frame, at)
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        let code = frame.fill.programs.code;
        staged_row_next(frame, cursor, listed(&code.pool, or_of(word).outputs))
    }
}
