//! Public stage_suite behavior over independent source fixtures.
use super::fixtures::{Refusal, Scenario, exercise, stage_missing};
use super::{interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
/// Stage a profile directory through the public use case.
fn stage(suite: &dyn SuitePort, directory: &Path, destination: &Path) -> Result<(), ReportsError> {
    StageSuiteUseCaseImpl::new(suite).stage_suite(
        directory,
        destination,
        &[],
        ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
    )
}
/// A fixture scenario, or a profile directory that does not exist.
#[derive(Clone, Copy, Debug)]
enum Operation {
    Fixture(Scenario),
    Missing,
}
type Input = Operation;
type Value = ();
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("subset"),
        word: Word::Succeeds,
        input: || Operation::Fixture(Scenario::Subset),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("workload"),
        word: Word::Succeeds,
        input: || Operation::Fixture(Scenario::Workload),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("supporting"),
        word: Word::Succeeds,
        input: || Operation::Fixture(Scenario::Supporting),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("discrepancy"),
        word: Word::Succeeds,
        input: || Operation::Fixture(Scenario::Discrepancy),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing"),
        word: Word::Refuses,
        input: || Operation::Missing,
        expect: Expect::Error(Refusal::MissingProfile),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Operation::Fixture(scenario) => {
            exercise(scenario);
            Ok(())
        }
        Operation::Missing => stage_missing(stage),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
