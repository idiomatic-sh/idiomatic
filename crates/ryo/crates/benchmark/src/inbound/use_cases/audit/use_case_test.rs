//! Coverage gates require the entire pinned default inventory.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::service::require_complete;
use crate::domain::suite::types::audit::value::Audit;
type Input = Audit;
type Value = ();
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Incomplete,
    Empty,
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
    Case {
        name: CaseName("empty internal audit is refused"),
        word: Word::Refuses,
        input: || Audit {
            implemented: Vec::new(),
            missing: Vec::new(),
        },
        expect: Expect::Error(Failure::Empty),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    require_complete(&input).map_err(|error| match error {
        Error::Invalid(message)
            if message == "default coverage incomplete: 1 missing rules: fixture" =>
        {
            Failure::Incomplete
        }
        Error::Invalid(message) if message == "empty audit is not default coverage" => {
            Failure::Empty
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
