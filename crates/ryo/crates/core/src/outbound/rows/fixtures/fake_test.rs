//! The `rows` port's fake runs the port's contract, then ends the answer as it was told to: well, closed or failed.

use super::super::contract::check;
use super::{Ends, RowsFake};
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::rows::errors::RowsError;
use crate::outbound::rows::interface::RowsPort as _;
use crate::outbound::rows::types::Summary;

/// How the fake is told to end the answer.
type Input = Ends;
/// Whether the answer ended.
type Value = ();
type Failure = RowsError;

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
        expect: Expect::Error(RowsError::Closed),
    },
    Case {
        name: CaseName("a_fake_told_to_fail_fails_the_answer"),
        word: Word::Fails,
        input: || Ends::Failed,
        expect: Expect::Refusal(|error| selects(matches!(error, RowsError::Failed { .. }))),
    },
];

fn subject(ends: Input) -> Result<Value, Failure> {
    let fake = RowsFake {
        ends,
        ..RowsFake::default()
    };
    fake.end(Summary::default(), &[])
}

#[test]
fn succeeds() {
    check(&RowsFake::default());
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
