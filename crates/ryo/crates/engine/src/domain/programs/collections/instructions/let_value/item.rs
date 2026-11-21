//! `Let`: one register written from a register, a constant or the scanned file.

use super::super::interface::{Flow, InstructionsItem, StepFrame};
use crate::domain::code::constants::{CONSTANT, THE_FILE};
use crate::domain::code::deciding::service::let_value_of;
use crate::domain::code::types::{CodeEntry, CodeWord, ConstantNumber};
use crate::domain::programs::operands::service::{cell, constant, file_cell, set};
use crate::domain::steps::types::Register;

/// The `Let` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LetValue;

impl InstructionsItem for LetValue {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let read = let_value_of(word);
        let value = if read.flag.0 & THE_FILE.0 != 0 {
            file_cell(frame)
        } else if read.flag.0 & CONSTANT.0 != 0 {
            constant(frame, ConstantNumber(read.from.0))
        } else {
            cell(frame, Register(read.from.0))
        };
        set(frame, read.out, value);
        Flow::Next
    }
}
