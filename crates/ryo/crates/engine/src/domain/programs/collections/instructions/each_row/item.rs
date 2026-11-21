//! `EachRow`: every row of a source, each bound as its handle. A made table's rows are iterated here; a filled
//! relation's and the node table's come through the fill side's seam.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::anchoring::service::each_row_made_of;
use crate::domain::code::types::{CodeEntry, CodeWord, Op};
use crate::domain::findings::types::InstructionCount;
use crate::domain::programs::operands::service::{after, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};
use ryo_language::domain::facts::types::CellWord;

/// The `EachRow` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EachRow;

impl InstructionsItem for EachRow {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, at: CodeEntry) -> Flow {
        if word.op != Op::EachRowMade {
            frame.state.work.unbuilt =
                InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
            return Flow::Back;
        }
        let read = each_row_made_of(word);
        let rows = frame
            .state
            .tables
            .get(usize::try_from(read.table.0).unwrap_or(usize::MAX))
            .map_or(0, |slot| slot.rows.count.0);
        frame.state.frames.push(IterationCursor {
            word: at,
            at: FramePlace(0),
            end: FramePlace(u32::try_from(rows).unwrap_or(u32::MAX)),
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
        set(
            frame,
            each_row_made_of(word).out,
            CellWord(u64::from(cursor.at.0)),
        );
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(after(cursor.word))
    }
}
