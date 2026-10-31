//! An engine refuses a corpus with no candidate rules rather than reporting success.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::Error;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
type Input = ();
type Value = ();
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    NoQueries,
    Unexpected,
}
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("missing candidate rules refuse evaluation"),
    word: Word::Refuses,
    input: || (),
    expect: Expect::Error(Failure::NoQueries),
}];
pub fn check(adapter: &impl EnginePort) {
    let temporary = tempfile::tempdir().expect("rules directory");
    let subject = |_| {
        adapter
            .run(WorkerRequest {
                root: temporary.path().into(),
                rules: temporary.path().into(),
                files: vec!["input.txt".into()],
                cache: None,
            })
            .map(|_| ())
            .map_err(|error| match error {
                Error::Invalid(message) if message == "no candidate queries" => Failure::NoQueries,
                _ => Failure::Unexpected,
            })
    };
    run(CASES, Word::Refuses, subject);
}

type Failure = Refusal;
