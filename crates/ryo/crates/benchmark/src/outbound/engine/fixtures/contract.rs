//! An engine refuses a corpus with no candidate rules rather than reporting success.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
type Input = ();
type Value = ();
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Canonical contract failure classifications.
pub enum Refusal {
    /// The candidate has no rules to evaluate.
    NoQueries,
    /// The implementation returned an unexpected failure category.
    Unexpected,
}
/// Named observations every implementation must satisfy.
pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("missing candidate rules refuse evaluation"),
    word: Word::Refuses,
    input: || (),
    expect: Expect::Error(Failure::NoQueries),
}];
/// Run the shared observations against the supplied implementation.
///
/// # Panics
///
/// Panics when fixture setup fails or the implementation violates a contract observation.
pub fn check(adapter: &impl EnginePort) {
    let temporary = tempfile::tempdir().expect("rules directory");
    let subject = |_| {
        adapter
            .run(WorkerRequest {
                root: temporary.path().into(),
                rules: temporary.path().into(),
                files: vec![ryo::linter::RelativePath("input.txt".into())],
                state: crate::domain::suite::types::WorkerState::Fresh,
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
