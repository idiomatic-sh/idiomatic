//! An engine refuses a corpus with no candidate rules rather than reporting success.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::interface::EnginePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
/// No variable input: this contract probes one fixed malformed request.
type Input = Scenario;
#[derive(Clone, Copy)]
pub enum Scenario {
    NoQueries,
    Loaded,
}
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
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("missing candidate rules refuse evaluation"),
        word: Word::Refuses,
        input: || Scenario::NoQueries,
        expect: Expect::Error(Failure::NoQueries),
    },
    Case {
        name: CaseName("loaded session owns rules through execution"),
        word: Word::Succeeds,
        input: || Scenario::Loaded,
        expect: Expect::Value(()),
    },
];
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
    let subject = |scenario| {
        let request = WorkerRequest {
                    performance: Default::default(),
                    selected_rules: None,
                    rule_observations: None,
                    rule_context: None,
                    project_search_roots: None,
                    given_project_inventory: Default::default(),
                    project_context_mode: ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(
                    ),
                    context_manifest: None,
                    root: temporary.path().into(),
                    rules: temporary.path().into(),
                    query_modules: std::collections::BTreeMap::default(),
                    rule_applicability: std::collections::BTreeMap::default(),
                    parser_configuration: None,
                    source_language: None,
                    files: vec![ryo::linter::RelativePath("input.txt".into())],
                    state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
                };
        if matches!(scenario, Scenario::Loaded) {
            let mut request = request;
            let id = ryo::linter::RuleId::try_from("R1").expect("rule identity");
            std::fs::create_dir(temporary.path().join("R1")).expect("rule directory");
            std::fs::write(temporary.path().join("R1/query.ryoql"), "let missing = select path as file, span from \"filesystem.read-failure\"; emit missing subject file key(file);").expect("query");
            let boundary = temporary.path().join("snapshot");
            request.context_manifest = Some(
                crate::outbound::context::adapters::local::adapter::materialize(
                    &boundary,
                    &ryo_benchmark_contracts::domain::context::types::CaseContext {
                        logical_root: "project".into(),
                        auxiliary_files: Default::default(),
                        missing: vec!["project/missing.py".into()],
                    },
                    &Default::default(),
                )
                .expect("sealed absence"),
            );
            request.project_context_mode = ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::ListedDisk;
            request.root = boundary.join("project");
            request.files = vec![ryo::linter::RelativePath("missing.py".into())];
            request.selected_rules = Some(vec![id.clone()]);
            let mut session = adapter.load(&request).map_err(|_| Failure::Unexpected)?;
            assert_eq!(session.rules_mut().len(), 1);
            assert!(session.frontend_rule().is_none());
            assert!(
                session
                    .configuration_files()
                    .map_err(|_| Failure::Unexpected)?
                    .is_empty()
            );
            std::fs::remove_file(temporary.path().join("R1/query.ryoql")).expect("loaded snapshot");
            let inputs = crate::outbound::engine::types::EngineConfiguration {
                selected_missing: request.files.iter().cloned().collect(),
                selected_rules: [id.clone()].into_iter().collect(),
                selected_pairs: 1,
                hosted: Vec::new(),
            };
            let report = session.execute(inputs).map_err(|_| Failure::Unexpected)?;
            assert_eq!(report.files, request.files);
            assert_eq!(
                report.diagnostics,
                vec![
                    ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding {
                        rule: id,
                        file: request.files[0].clone(),
                        start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0),
                        end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(0)
                    }
                ]
            );
            return Ok(());
        }
        adapter
            .load(&request)
            .map(|_| ())
            .map_err(|error| match error {
                Error::Invalid(message) if message == "no candidate queries" => Failure::NoQueries,
                Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => {
                    Failure::Unexpected
                }
            })
    };
    run(CASES, Word::Refuses, &subject);
    run(CASES, Word::Succeeds, subject);
}

/// Classified engine refusal.
type Failure = Refusal;
