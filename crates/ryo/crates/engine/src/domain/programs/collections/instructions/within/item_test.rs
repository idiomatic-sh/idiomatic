//! The `Within` word's cases: the candidate goes on when the loop found the ancestor, and ends when it did not.

use super::item::Within;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{CodeEntry, WordFlag};
use crate::domain::code::walking::service::within;
use crate::domain::code::walking::types::WithinWord;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use crate::domain::steps::types::Register;
use ryo_relations::domain::relations::types::Count;

/// How many rows the loop's answer holds: one when the ancestor was found.
type Input = usize;
/// Whether the candidate goes on (1) or ends (0).
type Value = u8;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_ancestor_found_lets_the_candidate_go_on"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("no_ancestor_found_ends_the_candidate"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value(0),
    },
];

fn subject(rows: Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(2);
    held.state.answer = BodyAnswer {
        first: Count(0),
        rows: Count(rows),
    };
    let word = within(WithinWord {
        flag: WordFlag(0),
        ancestor: Register(0),
        descendant: Register(1),
    });
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        Within::run(frame, word, CodeEntry(1))
    });
    Ok(u8::from(flow == Flow::Next))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
