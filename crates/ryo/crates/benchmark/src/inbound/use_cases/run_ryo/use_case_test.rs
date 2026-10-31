//! The worker forwards successful reports and dependency failures.
use super::{interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::fixtures::fake::EngineFake;
use std::sync::Arc;
type Input = bool;
type Value = bool;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    NoQueries,
    Unexpected,
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("worker preserves submitted files"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("worker preserves engine refusal"),
        word: Word::Fails,
        input: || true,
        expect: Expect::Error(Failure::NoQueries),
    },
];
fn subject(refuses: Input) -> Result<Value, Failure> {
    let worker = RunRyoUseCaseImpl::new(Arc::new(EngineFake { refuses }));
    worker
        .run_ryo(WorkerRequest {
            root: ".".into(),
            rules: "rules".into(),
            files: vec!["input.txt".into()],
            cache: None,
        })
        .map(|report| report.files == ["input.txt"])
        .map_err(|error| match error {
            Error::Invalid(message) if message == "no candidate queries" => Failure::NoQueries,
            _ => Failure::Unexpected,
        })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

type Failure = Refusal;
