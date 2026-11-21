//! The `Yield` word's cases: the row appended is the cells of its registers, in order; one register, several, or none.

use super::item::YieldRow;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::controlling::service::yield_row;
use crate::domain::code::controlling::types::YieldWord;
use crate::domain::code::records::service::push_list;
use crate::domain::code::types::{CodeEntry, ProgramCode, Short, WordSlot};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use ryo_language::domain::facts::types::CellWord;
use std::borrow::Cow;

/// The registers of the row.
type Input = &'static [u32];
/// The rows yielded so far, after the word: one cell was there before it.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the registers hold 5, 6 and 7.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_of_one_register"),
        word: Word::Succeeds,
        input: || &[1],
        expect: Expect::Value(Cow::Borrowed(&[40, 6])),
    },
    Case {
        name: CaseName("a_row_of_two_registers_in_the_words_order"),
        word: Word::Succeeds,
        input: || &[2, 0],
        expect: Expect::Value(Cow::Borrowed(&[40, 7, 5])),
    },
    Case {
        name: CaseName("a_row_of_no_cell_appends_nothing"),
        word: Word::Succeeds,
        input: || &[],
        expect: Expect::Value(Cow::Borrowed(&[40])),
    },
];

fn subject(registers: Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let slots: Vec<WordSlot> = registers.iter().map(|each| WordSlot(*each)).collect();
    let row = match registers {
        [one] => WordSlot(*one),
        _ => WordSlot(push_list(&mut code.pool, &slots).0),
    };
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(3);
    held.state.registers = vec![CellWord(5), CellWord(6), CellWord(7)];
    held.state.yielded.push(CellWord(40));
    let word = yield_row(YieldWord {
        cells: Short(u16::try_from(registers.len()).unwrap_or(u16::MAX)),
        row,
    });
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        YieldRow::run(frame, word, CodeEntry(1))
    });
    if flow == Flow::Back {
        Ok(Cow::Owned(
            held.state.yielded.iter().map(|cell| cell.0).collect(),
        ))
    } else {
        Err(())
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
