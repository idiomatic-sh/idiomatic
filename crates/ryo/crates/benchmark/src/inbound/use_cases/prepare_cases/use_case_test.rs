//! Public prepare_cases behavior over independent source fixtures.
use super::fixtures::{Refusal, Scenario, exercise};
use super::{interface::PrepareCasesUseCase as _, use_case::PrepareCasesUseCaseImpl};
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Scenario;
type Value = usize;
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("missing"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(Refusal::MissingSource),
    },
];
fn prepare(suite: &dyn SuitePort, rules: &Path, scratch: &Path) -> Result<usize, ReportsError> {
    PrepareCasesUseCaseImpl::new(suite, &crate::outbound::context::factory::local())
        .prepare_cases(rules, scratch)
        .map(|cases| {
            assert_eq!(
                cases
                    .iter()
                    .map(|case| case.expected.diagnostics.len())
                    .collect::<Vec<_>>(),
                [1, 0]
            );
            cases.len()
        })
}
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input, prepare)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
