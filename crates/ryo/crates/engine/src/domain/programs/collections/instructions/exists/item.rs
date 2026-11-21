//! `Exists`: the candidate goes on when its body has a row, or with the absent flag when it has none. The loop asks the
//! body and leaves its answer in the state; this item reads the answer.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::constants::ABSENT;
use crate::domain::code::controlling::service::exists_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::frames::types::StepFrame;

/// The `Exists` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Exists;

impl InstructionsItem for Exists {
    #[inline]
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let has = frame.state.answer.rows.0 > 0;
        let wanted = exists_of(word).flag.0 & ABSENT.0 == 0;
        if has == wanted {
            Flow::Next
        } else {
            Flow::Back
        }
    }
}
