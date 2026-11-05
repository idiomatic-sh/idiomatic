//! Public stage_suite behavior over independent source fixtures.
use super::fixtures::{Refusal, Scenario, exercise};
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Scenario;
type Value = ();
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("subset"),
        word: Word::Succeeds,
        input: || Scenario::Subset,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("workload"),
        word: Word::Succeeds,
        input: || Scenario::Workload,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("supporting"),
        word: Word::Succeeds,
        input: || Scenario::Supporting,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("discrepancy"),
        word: Word::Succeeds,
        input: || Scenario::Discrepancy,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(Refusal::MissingProfile),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
