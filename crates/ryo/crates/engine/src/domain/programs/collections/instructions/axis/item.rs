//! `Axis`: each element along a tree axis from a node, bound to the word's register in turn. The loop stages the elements
//! the walk reaches and leaves where they are in the state; this item iterates them and gives the room back when they are
//! done.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::code::walking::service::axis_of;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{after, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};

/// The `Axis` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Axis;

impl InstructionsItem for Axis {
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
        let place = first.saturating_add(usize::try_from(cursor.at.0).unwrap_or(usize::MAX));
        let reached = frame
            .state
            .staged
            .get(place)
            .copied()
            .filter(|_| cursor.at.0 < cursor.end.0);
        let Some(element) = reached else {
            frame.state.staged.truncate(first);
            return None;
        };
        set(frame, axis_of(word).out, element);
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(after(cursor.word))
    }
}
