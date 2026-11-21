//! `Let`: one register written from a register, a constant or the scanned file.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::constants::{CONSTANT, THE_FILE};
use crate::domain::code::deciding::service::let_value_of;
use crate::domain::code::types::{CodeEntry, CodeWord, ConstantNumber};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::{cell, constant, file_cell, set};
use crate::domain::steps::types::Register;

/// The `Let` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LetValue;

impl InstructionsItem for LetValue {
    #[inline]
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let read = let_value_of(word);
        let value = if read.flag.0 & THE_FILE.0 != 0 {
            // SPIKE (ryo-fmx3.1, fault L): the register holds the file: a test of a text reads the file's path of it.
            if !frame.state.file_registers.contains(&read.out.0) {
                let _new: bool = frame.state.file_registers.insert(read.out.0);
            }
            file_cell(frame)
        } else if read.flag.0 & CONSTANT.0 != 0 {
            constant(frame, ConstantNumber(read.from.0))
        } else {
            if !frame.state.file_registers.is_empty() && frame.state.file_registers.contains(&read.from.0) {
                let _new: bool = frame.state.file_registers.insert(read.out.0);
            }
            cell(frame, Register(read.from.0))
        };
        set(frame, read.out, value);
        Flow::Next
    }
}
