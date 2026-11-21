//! The `Read` word's cases: a made table's cell by the row's handle and the column, read through the seam; where the
//! column lies is held for the file from the word's first read that finds it, and for that file alone.

use super::item::Read;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::deciding::service::read_made;
use crate::domain::code::deciding::types::ReadMadeWord;
use crate::domain::code::types::CodeEntry;
use crate::domain::indexes::types::FileStamp;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Held, Idle, framed, held, table};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{MadeColumn, MadeTableId, Register};
use ryo_language::domain::facts::types::CellWord;
use std::borrow::Cow;

/// What happens around the word's reads.
#[derive(Clone, Copy, Debug)]
enum Reads {
    /// The table is made; the word reads once.
    Once,
    /// The table is made; the word reads twice in the file.
    Twice,
    /// The table is made and the word reads; then the next file begins, for which the table is not made, and it reads.
    NextFile,
    /// The word reads before the table is made, the table is made, and it reads again.
    MadeLater,
    /// The table is made and the word reads; then the next file begins, the table is made with other rows, and it reads.
    MadeAgain,
}

/// The row's handle, the column read, and what happens around the reads.
type Input = (u64, u16, Reads);
/// The register after each read, then how many words were counted as not run.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: table 1 holds the rows (10, 11) and (20, 21); made again, (30, 31) and (40, 41).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_second_rows_second_column"),
        word: Word::Succeeds,
        input: || (1, 1, Reads::Once),
        expect: Expect::Value(Cow::Borrowed(&[21, 0])),
    },
    Case {
        name: CaseName("the_first_rows_first_column"),
        word: Word::Succeeds,
        input: || (0, 0, Reads::Once),
        expect: Expect::Value(Cow::Borrowed(&[10, 0])),
    },
    Case {
        name: CaseName("a_second_read_in_the_file_reads_the_held_column"),
        word: Word::Succeeds,
        input: || (1, 1, Reads::Twice),
        expect: Expect::Value(Cow::Borrowed(&[21, 21, 0])),
    },
    Case {
        name: CaseName("a_table_not_made_for_the_next_file_reads_no_cell_of_the_last"),
        word: Word::Succeeds,
        input: || (1, 1, Reads::NextFile),
        expect: Expect::Value(Cow::Borrowed(&[21, 0, 0])),
    },
    Case {
        name: CaseName("a_read_before_the_table_is_made_holds_nothing"),
        word: Word::Succeeds,
        input: || (1, 1, Reads::MadeLater),
        expect: Expect::Value(Cow::Borrowed(&[0, 21, 0])),
    },
    Case {
        name: CaseName("the_next_files_table_is_read_in_the_next_file"),
        word: Word::Succeeds,
        input: || (1, 0, Reads::MadeAgain),
        expect: Expect::Value(Cow::Borrowed(&[20, 40, 0])),
    },
];

/// Runs the word once and answers the register it wrote.
fn read_once(programs: &ProgramsEngine, case: &mut Held) -> Result<u64, Failure> {
    case.state.registers[1] = CellWord(5);
    let word = programs.code.words[1];
    let (flow, _found) = framed(programs, case, &Idle, |frame| {
        Read::run(frame, word, CodeEntry(1))
    });
    if flow == Flow::Next {
        Ok(case.state.registers[1].0)
    } else {
        Err(())
    }
}

fn subject((handle, column, reads): Input) -> Result<Value, Failure> {
    let word = read_made(ReadMadeWord {
        column: MadeColumn(column),
        row: Register(0),
        out: Register(1),
        table: MadeTableId(1),
    });
    // The word is the code's second word, so the worker's state holds a slot for its column.
    let mut programs = ProgramsEngine::default();
    programs.code.words = vec![word, word];
    let mut held = held(2);
    held.state.registers[0] = CellWord(handle);
    let first: &[&[u64]] = &[&[10, 11], &[20, 21]];
    let mut answers = Vec::new();
    if !matches!(reads, Reads::MadeLater) {
        table(&mut held, 1, 2, first);
    }
    answers.push(read_once(&programs, &mut held)?);
    match reads {
        Reads::Once => {}
        Reads::Twice => answers.push(read_once(&programs, &mut held)?),
        Reads::NextFile => {
            held.store.stamp = FileStamp(held.store.stamp.0 + 1);
            answers.push(read_once(&programs, &mut held)?);
        }
        Reads::MadeLater => {
            table(&mut held, 1, 2, first);
            answers.push(read_once(&programs, &mut held)?);
        }
        Reads::MadeAgain => {
            held.store.stamp = FileStamp(held.store.stamp.0 + 1);
            table(&mut held, 1, 2, &[&[30, 31], &[40, 41]]);
            answers.push(read_once(&programs, &mut held)?);
        }
    }
    answers.push(held.state.work.unbuilt.0);
    Ok(Cow::Owned(answers))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
