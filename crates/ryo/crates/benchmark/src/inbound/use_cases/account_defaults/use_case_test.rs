//! A tool is accounted for only when its audit authored rules.
use super::{interface::AccountDefaultsUseCase as _, use_case::AccountDefaultsUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::{admit_measurement::fixtures::tool, audit::fixtures::audited};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// Whether the audit authored the tool's one default.
type Input = bool;
type Value = ();
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Unexpected,
}
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an authored default is accounted for"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an audit without authored rules is refused"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(Refusal::Invalid),
    },
];

fn subject(authored: Input) -> Result<Value, Failure> {
    AccountDefaultsUseCaseImpl::new()
        .account_defaults(&audited(authored), &tool())
        .map_err(|error| match error {
            ReportsError::Invalid(_) => Refusal::Invalid,
            ReportsError::Io(_) | ReportsError::Json(_) | ReportsError::Toml(_) => {
                Refusal::Unexpected
            }
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
