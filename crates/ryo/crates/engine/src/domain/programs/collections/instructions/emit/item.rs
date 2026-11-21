//! `Emit`: one found row handed over: the cells its operands stand for, for the rule or for the rule's frontier entry.

use super::super::interface::{Flow, InstructionsItem};
use crate::domain::code::constants::FRONTIER;
use crate::domain::code::controlling::service::emit_of;
use crate::domain::code::records::service::operand_at;
use crate::domain::code::types::{CodeEntry, CodeWord, PoolAt};
use crate::domain::findings::types::{FoundCells, FoundTarget};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::operand;
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::plan_parts::types::InputNumber;

/// The `Emit` word.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Emit;

impl InstructionsItem for Emit {
    fn run(frame: &mut StepFrame<'_, '_>, word: CodeWord, _at: CodeEntry) -> Flow {
        let read = emit_of(word);
        let start = frame.state.key.len();
        for number in 0..u32::from(read.columns.0) {
            let at = PoolAt(read.operands.0.wrapping_add(number.wrapping_mul(2)));
            let value = operand_at(&frame.fill.programs.code.pool, at)
                .map_or(CellWord(0), |held| operand(frame, held));
            frame.state.key.push(value);
        }
        let target = if read.flag.0 & FRONTIER.0 == 0 {
            FoundTarget::Rule(read.rule)
        } else {
            FoundTarget::Frontier(
                read.rule,
                InputNumber(usize::try_from(read.input.0).unwrap_or(usize::MAX)),
            )
        };
        frame.found.found(
            target,
            FoundCells(frame.state.key.get(start..).unwrap_or_default()),
            &*frame.fill.words,
        );
        frame.state.key.truncate(start);
        Flow::Next
    }
}
