//! Coverage gates require the entire pinned default inventory.
use super::service::require_complete;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::types::Audit;
type Input = Audit;
type Value = ();
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Incomplete,
    Unexpected,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete defaults accepted"),
        word: Word::Succeeds,
        input: || Audit {
            implemented: vec!["fixture".into()],
            missing: Vec::new(),
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("one missing default prevents measurement"),
        word: Word::Refuses,
        input: || Audit {
            implemented: Vec::new(),
            missing: vec!["fixture".into()],
        },
        expect: Expect::Error(Failure::Incomplete),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    require_complete(&input).map_err(|error| match error {
        Error::Invalid(message)
            if message == "default coverage incomplete: 1 missing rules: fixture" =>
        {
            Failure::Incomplete
        }
        _ => Failure::Unexpected,
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;
