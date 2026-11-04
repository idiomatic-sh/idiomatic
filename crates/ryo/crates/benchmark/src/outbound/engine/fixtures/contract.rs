//! An engine refuses a corpus with no candidate rules rather than reporting success.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::diagnostics::errors::DiagnosticsError as Error;
use crate::domain::suite::types::WorkerRequest;
use crate::outbound::engine::interface::EnginePort;
/// No variable input: this contract probes one fixed malformed request.
type Input = ();
/// Successful completion has no payload.
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
#[expect(
    clippy::expect_used,
    reason = "fixture setup failures must fail the contract test"
)]
pub fn check(adapter: &impl EnginePort) {
    let temporary = tempfile::tempdir().expect("rules directory");
    let subject = |()| {
        adapter
            .run(
                WorkerRequest {
                    performance: Default::default(),
                    project_context_mode: crate::domain::suite::types::ProjectContextMode::default(
                    ),
                    context_manifest: None,
                    root: temporary.path().into(),
                    rules: temporary.path().into(),
                    query_modules: std::collections::BTreeMap::default(),
                    rule_applicability: std::collections::BTreeMap::default(),
                    parser_configuration: None,
                    source_language: None,
                    files: vec![ryo::linter::RelativePath("input.txt".into())],
                    state: crate::domain::suite::types::WorkerState::Fresh,
                },
                crate::outbound::engine::types::PreparedMissingInputs {
                    selected_missing: std::collections::BTreeSet::new(),
                },
            )
            .map(|_| ())
            .map_err(|error| match error {
                Error::Invalid(message) if message == "no candidate queries" => Failure::NoQueries,
                Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => {
                    Failure::Unexpected
                }
            })
    };
    run(CASES, Word::Refuses, subject);
}

/// Classified engine refusal.
type Failure = Refusal;
