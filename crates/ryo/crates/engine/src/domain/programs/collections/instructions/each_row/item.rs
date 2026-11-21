//! `EachRow`: every row of a source, each bound as its handle: a made table's and a filled relation's rows by number, the
//! node table's elements by id. The loop asks the fill side for the rows; this item iterates the range it is handed.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::anchoring::service::{
    each_row_filled_of, each_row_made_of, each_row_node_of,
};
use crate::domain::code::types::{CodeEntry, CodeWord, Op};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{after, node_tag, set};
use crate::domain::scratch::types::{FramePlace, IterationCursor};
use ryo_language::domain::facts::types::CellWord;

/// The `EachRow` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EachRow;

impl InstructionsItem for EachRow {
    #[inline]
    fn run(frame: &mut StepFrame<'_, '_>, _word: CodeWord, at: CodeEntry) -> Flow {
        let run = frame.state.rows.run;
        frame.state.frames.push(IterationCursor {
            word: at,
            at: FramePlace(u32::try_from(run.start.0).unwrap_or(u32::MAX)),
            end: FramePlace(u32::try_from(run.end.0).unwrap_or(u32::MAX)),
            beside: FramePlace(0),
        });
        Flow::Back
    }

    #[inline]
    fn next(
        frame: &mut StepFrame<'_, '_>,
        word: CodeWord,
        cursor: &mut IterationCursor,
    ) -> Option<CodeEntry> {
        if cursor.at.0 >= cursor.end.0 {
            return None;
        }
        let (out, tag) = if word.op == Op::EachRowNode {
            (each_row_node_of(word).out, node_tag(frame))
        } else if word.op == Op::EachRowFilled {
            (each_row_filled_of(word).out, CellWord(0))
        } else {
            (each_row_made_of(word).out, CellWord(0))
        };
        set(frame, out, CellWord(tag.0 | u64::from(cursor.at.0)));
        cursor.at = FramePlace(cursor.at.0.saturating_add(1));
        Some(after(cursor.word))
    }
}
