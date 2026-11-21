//! The `Axis` word's cases: the elements the loop staged are bound in turn, and the staged room is given back.

use super::item::Axis;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{CodeEntry, WordFlag};
use crate::domain::code::walking::service::axis;
use crate::domain::code::walking::types::AxisWord;
use crate::domain::programs::collections::instructions::interface::InstructionsItem;
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// The cells staged before the walk's, and the cells the walk staged.
type Input = (&'static [u64], &'static [u64]);
/// The cell bound at each turn; then 99, where the loop goes on (the entry after the word, once), and how many staged
/// cells are left.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_staged_element_is_bound_in_turn_and_the_room_given_back"),
        word: Word::Succeeds,
        input: || (&[40], &[7, 8, 9]),
        expect: Expect::Value(Cow::Borrowed(&[7, 8, 9, 99, 6, 1])),
    },
    Case {
        name: CaseName("a_walk_that_reached_nothing_binds_nothing"),
        word: Word::Succeeds,
        input: || (&[40, 41], &[]),
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 2])),
    },
];

fn subject((before, reached): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(2);
    held.state.staged = before
        .iter()
        .chain(reached)
        .map(|cell| CellWord(*cell))
        .collect();
    held.state.answer = BodyAnswer {
        first: Count(before.len()),
        rows: Count(reached.len()),
    };
    let word = axis(AxisWord {
        flag: WordFlag(0),
        direction: WordFlag(0),
        start: Register(0),
        out: Register(1),
    });
    let (bound, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let _flow = Axis::run(frame, word, CodeEntry(5));
        let mut cursor = frame.state.frames.pop().unwrap_or_default();
        let (mut bound, mut onward) = (Vec::new(), 0_u64);
        while let Some(next) = Axis::next(frame, word, &mut cursor) {
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
