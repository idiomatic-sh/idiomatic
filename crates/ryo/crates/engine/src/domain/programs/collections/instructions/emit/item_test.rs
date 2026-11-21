//! The `Emit` word's cases: the found row is the cells its operands stand for, in order, and the key room is left as it was.

use super::item::Emit;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::emit;
use crate::domain::code::controlling::types::EmitWord;
use crate::domain::code::records::service::{next_place, push_operand};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::types::{
    CodeEntry, ConstantNumber, ProgramCode, Short, WordFlag, WordSlot,
};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// How many of the three operands the row has.
type Input = u16;
/// The cells of the one row found, then 99 and how many cells the key room holds after.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the operands are register 1 (holding 30), constant 0 (which is 60) and the file (7).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_of_a_register_a_constant_and_the_file"),
        word: Word::Succeeds,
        input: || 3,
        expect: Expect::Value(Cow::Borrowed(&[30, 60, 7, 99, 1])),
    },
    Case {
        name: CaseName("a_row_of_no_cell_is_a_row"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(Cow::Borrowed(&[99, 1])),
    },
];

fn subject(columns: Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        constants: vec![CellWord(60)],
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let operands = next_place(&code.pool);
    push_operand(&mut code.pool, PooledOperand::Register(Register(1)));
    push_operand(&mut code.pool, PooledOperand::Constant(ConstantNumber(0)));
    push_operand(&mut code.pool, PooledOperand::File);
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(2);
    held.state.registers[1] = CellWord(30);
    held.state.key.push(CellWord(1));
    let word = emit(EmitWord {
        flag: WordFlag(0),
        columns: Short(columns),
        rule: SelectedRuleOrdinal(0),
        input: WordSlot(0),
        operands,
    });
    let (flow, found) = framed(&programs, &mut held, &Idle, |frame| {
        Emit::run(frame, word, CodeEntry(1))
    });
    if flow != Flow::Next || found.rows.len() != 1 {
        return Err(());
    }
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.push(99);
    all.push(u64::try_from(held.state.key.len()).unwrap_or(u64::MAX));
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
