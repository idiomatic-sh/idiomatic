//! `Nearest` over the tree: the candidate goes on with the nearest ancestor its condition holds of, or ends when none
//! holds. The loop walks and asks the condition, stages the element found and leaves where in the state; this item binds
//! it and gives the room back.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::code::walking::service::nearest_of;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::set;

/// The `Nearest` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Nearest;

impl InstructionsItem for Nearest {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let answer = frame.state.answer;
        let found = frame
            .state
            .staged
            .get(answer.first.0)
            .copied()
            .filter(|_| answer.rows.0 > 0);
        frame.state.staged.truncate(answer.first.0);
        match found {
            Some(element) => {
                set(frame, nearest_of(word).out, element);
                Flow::Next
            }
            None => Flow::Back,
        }
    }
}
