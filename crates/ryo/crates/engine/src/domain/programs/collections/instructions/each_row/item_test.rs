//! The `EachRow` word's cases: every row of a made table is bound once, in order, as its handle, and then no more.

use super::item::EachRow;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::service::each_row_made;
use crate::domain::code::anchoring::types::EachRowMadeWord;
use crate::domain::code::types::{CodeEntry, WordFlag};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{framed, state, table};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{MadeTableId, Register};
use std::borrow::Cow;

/// How many rows the table holds.
type Input = usize;
/// The handles bound, each followed by the word the loop goes on at; then 99 and how many cursors are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the word stands at place 4.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("three_rows_are_bound_in_order_and_then_no_more"),
        word: Word::Succeeds,
        input: || 3,
        expect: Expect::Value(Cow::Borrowed(&[0, 5, 1, 5, 2, 5, 99, 1])),
    },
    Case {
        name: CaseName("a_table_of_no_row_binds_nothing"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(Cow::Borrowed(&[99, 1])),
    },
];

fn subject(rows: Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = state(1);
    let all: Vec<&[u64]> = (0..rows).map(|_| &[8_u64][..]).collect();
    table(&mut held, 0, 1, &all);
    let word = each_row_made(EachRowMadeWord {
        flag: WordFlag(0),
        out: Register(0),
        table: MadeTableId(0),
    });
    let (bound, _found) = framed(&programs, &mut held, |frame| {
        let mut bound = Vec::new();
        if EachRow::run(frame, word, CodeEntry(4)) != Flow::Back {
            return bound;
        }
        let Some(mut cursor) = frame.state.frames.last().copied() else {
            return bound;
        };
        while let Some(onward) = EachRow::next(frame, word, &mut cursor) {
            bound.extend([frame.state.registers[0].0, u64::from(onward.0)]);
        }
        bound.push(99);
        bound.push(u64::try_from(frame.state.frames.len()).unwrap_or(u64::MAX));
        bound
    });
    Ok(Cow::Owned(bound))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
