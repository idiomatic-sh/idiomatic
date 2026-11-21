//! `Yield`: one row of a collected body: the cells of its registers, appended to the rows the running bodies yielded.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::controlling::service::yield_row_of;
use crate::domain::code::types::{CodeEntry, CodeWord, PoolAt};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, listed};
use crate::domain::steps::types::Register;

/// The `Yield` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct YieldRow;

impl InstructionsItem for YieldRow {
    #[inline]
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let read = yield_row_of(word);
        if read.cells.0 == 1 {
            let value = cell(frame, Register(read.row.0));
            frame.state.yielded.push(value);
        } else {
            for register in listed(&frame.fill.programs.code.pool, PoolAt(read.row.0)) {
                let value = cell(frame, Register(register.0));
                frame.state.yielded.push(value);
            }
        }
        Flow::Back
    }
}
