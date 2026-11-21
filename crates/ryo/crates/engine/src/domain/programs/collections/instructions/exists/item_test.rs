//! The `Exists` word's cases: the candidate goes on when the body has a row, or with the absent flag when it has none.

use super::item::Exists;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::ABSENT;
use crate::domain::code::controlling::service::exists;
use crate::domain::code::controlling::types::ExistsWord;
use crate::domain::code::types::{CodeEntry, MemoSite, PoolAt, Short, WordFlag};
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::BodyAnswer;
use ryo_relations::domain::relations::types::Count;

/// How many rows the body answered, and the word's flag.
type Input = (usize, WordFlag);
/// Whether the candidate goes on (1) or ends (0).
type Value = u8;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_body_with_a_row_lets_the_candidate_go_on"),
        word: Word::Succeeds,
        input: || (1, WordFlag(0)),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_body_with_no_row_ends_the_candidate"),
        word: Word::Succeeds,
        input: || (0, WordFlag(0)),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("absent_ends_the_candidate_when_the_body_has_a_row"),
        word: Word::Succeeds,
        input: || (1, ABSENT),
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("absent_lets_the_candidate_go_on_when_the_body_has_none"),
        word: Word::Succeeds,
        input: || (0, ABSENT),
        expect: Expect::Value(1),
    },
];

fn subject((rows, flag): Input) -> Result<Value, Failure> {
    let programs = ProgramsEngine::default();
    let mut held = held(1);
    held.state.answer = BodyAnswer {
        first: Count(0),
        rows: Count(rows),
    };
    let word = exists(ExistsWord {
        flag,
        keys: Short(0),
        body: PoolAt(0),
        key: PoolAt(0),
        memo: MemoSite(0),
    });
    let (flow, _found) = framed(&programs, &mut held, &Idle, |frame| {
        Exists::run(frame, word, CodeEntry(1))
    });
    Ok(u8::from(flow == Flow::Next))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
