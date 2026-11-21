//! The `Latest` word's cases: the handles the loop staged are bound in turn, and the staged room is given back.

use super::item::Latest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::service::latest;
use crate::domain::code::anchoring::types::LatestWord;
use crate::domain::code::types::{CodeEntry, PoolAt, Short, WordFlag, WordSlot};
use crate::domain::programs::collections::instructions::interface::InstructionsItem;
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// The cells staged before the read's, and the handles the read staged.
type Input = (&'static [u64], &'static [u64]);
/// The handle bound at each turn; then 99, where the loop goes on (the entry after the word, once), and how many staged
/// cells are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_staged_handle_is_bound_in_turn_and_the_room_given_back"),
        word: Word::Succeeds,
        input: || (&[40], &[2, 3]),
        expect: Expect::Value(Cow::Borrowed(&[2, 3, 99, 6, 1])),
    },
    Case {
        name: CaseName("a_read_of_no_row_binds_nothing"),
        word: Word::Succeeds,
        input: || (&[40], &[]),
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 1])),
    },
];

fn subject((before, read): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(2);
    held.state.staged = before
        .iter()
        .chain(read)
        .map(|cell| CellWord(*cell))
        .collect();
    held.state.answer = BodyAnswer {
        first: Count(before.len()),
        rows: Count(read.len()),
    };
    let word = latest(LatestWord {
        flag: WordFlag(0),
        keys: Short(0),
        out: Register(1),
        bound: WordSlot(0),
        operands: PoolAt(0),
    });
    let (bound, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let _flow = Latest::run(frame, word, CodeEntry(5));
        let mut cursor = frame.state.frames.pop().unwrap_or_default();
        let (mut bound, mut onward) = (Vec::new(), 0_u64);
        while let Some(next) = Latest::next(frame, word, &mut cursor) {
            bound.push(frame.state.registers.get(1).map_or(0, |cell| cell.0));
            onward = u64::from(next.0);
        }
        bound.extend([99, onward]);
        bound
    });
    let mut all = bound;
    all.push(u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX));
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
