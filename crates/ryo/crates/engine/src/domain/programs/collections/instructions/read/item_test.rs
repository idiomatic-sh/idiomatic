//! The `Read` word's cases: a made table's cell by the row's handle and the column, read through the seam.

use super::item::Read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::read_made;
use crate::domain::code::deciding::types::ReadMadeWord;
use crate::domain::code::types::CodeEntry;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held, table};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{MadeColumn, MadeTableId, Register};
use ryo_language::domain::facts::types::CellWord;

/// The row's handle and the column read.
type Input = (u64, u16);
/// The register written, and how many words were counted as not run.
type Value = (u64, u64);
type Failure = ();

/// The rows of this part's cases: table 1 holds the rows (10, 11) and (20, 21).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_second_rows_second_column"),
        word: Word::Succeeds,
        input: || (1, 1),
        expect: Expect::Value((21, 0)),
    },
    Case {
        name: CaseName("the_first_rows_first_column"),
        word: Word::Succeeds,
        input: || (0, 0),
        expect: Expect::Value((10, 0)),
    },
];

fn subject((handle, column): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(2);
    table(&mut held, 1, 2, &[&[10, 11], &[20, 21]]);
    held.state.registers[0] = CellWord(handle);
    held.state.registers[1] = CellWord(5);
    let word = read_made(ReadMadeWord {
        column: MadeColumn(column),
        row: Register(0),
        out: Register(1),
        table: MadeTableId(1),
    });
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        Read::run(frame, word, CodeEntry(1))
    });
    if flow == Flow::Next {
        Ok((held.state.registers[1].0, held.state.work.unbuilt.0))
    } else {
        Err(())
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
