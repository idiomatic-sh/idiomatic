//! `Or`: every row of its bodies, each bound to the word's outputs in turn. The loop stages the bodies' rows and leaves
//! where they are in the state; this item iterates them and gives the room back when they are done.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::controlling::service::or_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::operands::service::{after, listed, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;

/// The `Or` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Or;

impl InstructionsItem for Or {
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        let answer = frame.state.answer;
        frame.state.frames.push(IterationCursor {
            word: at,
            at: FramePlace(0),
            end: FramePlace(u32::try_from(answer.rows.0).unwrap_or(u32::MAX)),
            beside: FramePlace(u32::try_from(answer.first.0).unwrap_or(u32::MAX)),
        });
        Flow::Back
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        let first = usize::try_from(cursor.beside.0).unwrap_or(usize::MAX);
        if cursor.at.0 >= cursor.end.0 {
            frame.state.staged.truncate(first);
            return None;
        }
        let outputs = listed(&frame.programs.code.pool, or_of(word).outputs);
        let row = first.saturating_add(
            usize::try_from(cursor.at.0)
                .unwrap_or(usize::MAX)
                .saturating_mul(outputs.len()),
        );
        for (place, output) in outputs.iter().enumerate() {
            let value = match frame.state.staged.get(row.saturating_add(place)) {
                Some(held) => *held,
                None => CellWord(0),
            };
            set(frame, Register(output.0), value);
        }
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(after(cursor.word))
    }
}
