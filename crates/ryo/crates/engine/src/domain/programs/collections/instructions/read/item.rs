//! `Read`: one cell of a row, by the row's handle, written to a register. A made table's row is read here; a filled
//! relation's and the node table's are read through the fill side's seam.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::deciding::service::read_made_of;
use crate::domain::code::types::{CodeEntry, CodeWord, Op};
use crate::domain::findings::types::InstructionCount;
use crate::domain::made_tables::service::row;
use crate::domain::programs::operands::service::{cell, set};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;

/// The `Read` words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Read;

impl InstructionsItem for Read {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        if word.op != Op::ReadMade {
            frame.state.work.unbuilt =
                InstructionCount(frame.state.work.unbuilt.0.saturating_add(1));
            return Flow::Back;
        }
        let read = read_made_of(word);
        let handle = cell(frame, read.row);
        let value = frame
            .state
            .tables
            .get(usize::try_from(read.table.0).unwrap_or(usize::MAX))
            .and_then(|slot| {
                row(
                    &slot.rows,
                    Count(usize::try_from(handle.0).unwrap_or(usize::MAX)),
                )
                .get(usize::from(read.column.0))
            })
            .map_or(CellWord(0), |held| *held);
        set(frame, read.out, value);
        Flow::Next
    }
}
