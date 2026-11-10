//! The `row_sink` port's fake runs the port's contract, then ends the answer as it was told to: well, closed or failed.

use super::super::contract::check;
use super::{Ends, RowSinkFake};
use crate::cases::{Case, CaseName, Expect, Matched, Word, run, selects};
use crate::outbound::row_sink::errors::RowSinkError;
use crate::outbound::row_sink::interface::RowSinkPort as _;
use crate::outbound::row_sink::types::AnswerCounts;

/// How the fake is told to end the answer.
type Input = Ends;
/// Whether the answer ended.
type Value = ();
type Failure = RowSinkError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fake_told_to_end_well_ends_the_answer"),
        word: Word::Succeeds,
        input: || Ends::Well,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_fake_told_its_reader_left_closes_the_answer"),
        word: Word::Refuses,
        input: || Ends::Closed,
        expect: Expect::Error(RowSinkError::Closed),
    },
    Case {
        name: CaseName("a_fake_told_to_fail_fails_the_answer"),
        word: Word::Fails,
        input: || Ends::Failed,
        expect: Expect::Refusal(|error| {
            selects(Matched(matches!(error, RowSinkError::Failed { .. })))
        }),
    },
];

fn subject(ends: Input) -> Result<Value, Failure> {
    let fake = RowSinkFake {
        ends,
        ..RowSinkFake::default()
    };
    fake.end(AnswerCounts::default(), &[])
}

#[test]
fn succeeds() {
    check(&RowSinkFake::default());
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
