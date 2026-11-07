//! Public stage_suite behavior over independent source fixtures.
use super::fixtures::{Refusal, Scenario, exercise};
use super::{interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl};
use crate::outbound::suite::factory::local;
use crate::cases::{Case, CaseName, Expect, Word, run};
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
        Operation::Missing => {
            let root = tempfile::tempdir().unwrap();
            let destination = root.path().join("snapshot");
            let suite = local();
            let result = StageSuiteUseCaseImpl::new(&suite).stage_suite(
                &root.path().join("missing"),
                &destination,
                &[],
                ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
            );
            assert_eq!(
                std::fs::symlink_metadata(&destination).unwrap_err().kind(),
                std::io::ErrorKind::NotFound
            );
            result.map_err(|error| match error {
                ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(error)
                    if error.kind() == std::io::ErrorKind::NotFound =>
                {
                    Refusal::MissingProfile
                }
                _ => Refusal::Unexpected,
            })
        }
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
