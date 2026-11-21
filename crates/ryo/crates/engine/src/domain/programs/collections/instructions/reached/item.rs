//! `Closure`: each element reached over one or more edges from a seed, bound to the word's register in turn. The loop
//! walks the edges and stages the elements, and leaves where they are in the state; this item iterates them and gives
//! the room back when they are done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::code::walking::service::closure_of;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{staged_cursor, staged_next};
use crate::domain::scratch::types::IterationCursor;

/// The `Closure` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Closure;

impl InstructionsItem for Closure {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        staged_cursor(frame, at)
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        staged_next(frame, cursor, closure_of(word).out)
    }
}
