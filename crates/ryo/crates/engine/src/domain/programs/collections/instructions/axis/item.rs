//! `Axis`: each element along a tree axis from a node, bound to the word's register in turn. The loop stages the elements
//! the walk reaches and leaves where they are in the state; this item iterates them and gives the room back when they are
//! done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::code::walking::service::axis_of;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{staged_cursor, staged_next};
use crate::domain::scratch::types::IterationCursor;

/// The `Axis` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis;

impl InstructionsItem for Axis {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        staged_cursor(frame, at)
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        staged_next(frame, cursor, axis_of(word).out)
    }
}
