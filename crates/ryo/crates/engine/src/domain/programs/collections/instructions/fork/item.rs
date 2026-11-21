//! `Fork`: the programs that share the words before it, each run in turn from the registers as they stand.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::controlling::service::fork_of;
use crate::domain::code::types::{CodeEntry, CodeWord};
use crate::domain::programs::operands::service::listed;
use crate::domain::scratch::types::{FramePlace, IterationCursor};

/// The `Fork` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fork;

impl InstructionsItem for Fork {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) -> Flow {
        frame.state.frames.push(IterationCursor {
            word: at,
            at: FramePlace(0),
            end: FramePlace(u32::from(fork_of(word).slots.0)),
            beside: FramePlace(0),
        });
        Flow::Back
    }

    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        if cursor.at.0 >= cursor.end.0 {
            return None;
        }
        let entry = listed(&frame.programs.code.pool, fork_of(word).entries)
            .get(usize::try_from(cursor.at.0).unwrap_or(usize::MAX))
            .map(|slot| CodeEntry(slot.0))?;
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(entry)
    }
}
