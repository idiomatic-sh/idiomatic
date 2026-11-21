//! `Within`: the candidate goes on when one node is a proper ancestor of another. The loop walks the descendant's
//! ancestors and leaves its answer in the state; this item reads the answer.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;

/// The `Within` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Within;

impl InstructionsItem for Within {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, _at: CodeEntry) -> Flow {
        if frame.state.answer.rows.0 > 0 {
            Flow::Next
        } else {
            Flow::Back
        }
    }
}
