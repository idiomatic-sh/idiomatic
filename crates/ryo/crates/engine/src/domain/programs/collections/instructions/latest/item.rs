//! `Latest`: each row of an ordered table nearest a bound in a key's group, its handle bound to the word's register in
//! turn. The loop reads the group through the fill side's seam, stages the handles and leaves where they are in the
//! state; this item iterates them and gives the room back when they are done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::anchoring::service::latest_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{staged_cursor, staged_next};
use crate::domain::scratch::types::IterationCursor;

/// The `Latest` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Latest;

impl InstructionsItem for Latest {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        staged_cursor(frame, at)
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        staged_next(frame, cursor, latest_of(word).out)
    }
}
