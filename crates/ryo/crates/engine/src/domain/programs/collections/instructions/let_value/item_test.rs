//! The `Let` word's cases: the register is written from a register, from a constant, or as the scanned file.

use super::item::LetValue;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::{CONSTANT, THE_FILE};
use crate::domain::code::deciding::service::let_value;
use crate::domain::code::deciding::types::LetWord;
use crate::domain::code::types::{CodeEntry, ProgramCode, WordFlag, WordSlot};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{framed, state};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;

/// The flag of the word and what its `from` slot holds.
type Input = (WordFlag, u32);
/// The register written, after the word.
type Value = u64;
type Failure = ();

/// The rows of this part's cases: register 0 holds 40, constant 1 is 50, the file is 7.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("from_a_register"),
        word: Word::Succeeds,
        input: || (WordFlag(0), 0),
        expect: Expect::Value(40),
    },
    Case {
        name: CaseName("from_a_constant"),
        word: Word::Succeeds,
        input: || (CONSTANT, 1),
        expect: Expect::Value(50),
    },
    Case {
        name: CaseName("the_scanned_file"),
        word: Word::Succeeds,
        input: || (THE_FILE, 0),
        expect: Expect::Value(7),
    },
];

fn subject((flag, from): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine {
        code: ProgramCode {
            constants: vec![CellWord(49), CellWord(50)],
            ..ProgramCode::default()
        },
        ..ProgramsEngine::default()
    };
    let mut held = state(2);
    held.registers[0] = CellWord(40);
    let word = let_value(LetWord {
        flag,
        from: WordSlot(from),
        out: Register(1),
    });
    let (flow, _found) = framed(&programs, &mut held, |frame| {
        LetValue::run(frame, word, CodeEntry(1))
    });
    if flow == Flow::Next {
        Ok(held.registers[1].0)
    } else {
        Err(())
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
