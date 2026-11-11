//! Selected missing inputs are prepared once before the SDK adapter receives the request.
use super::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::context::types::ContextManifest;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[cfg(test)]
mod sdk;

#[derive(Clone, Copy)]
enum Scenario {
    Sealed,
    UndeclaredMissing,
    WrongMode,
    TraceSessions,
}
type Input = Scenario;
type Value = usize;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "actual trace guards survive loading configuration SDK errors and abandonment",
        ),
        word: Word::Succeeds,
        input: || Scenario::TraceSessions,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("only selected sealed absences are passed to the SDK adapter"),
        word: Word::Succeeds,
        input: || Scenario::Sealed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("unsealed requested absence cannot enter the SDK"),
        word: Word::Refuses,
        input: || Scenario::UndeclaredMissing,
        expect: Expect::Error("context source seal differs from requested inventory"),
    },
    Case {
        name: CaseName("isolated mode cannot inherit a disk context"),
        word: Word::Refuses,
        input: || Scenario::WrongMode,
        expect: Expect::Error("worker context mode requires a matching explicit manifest"),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(scenario, Scenario::TraceSessions) {
        super::fixtures::trace_sessions();
        return Ok(1);
    }
    let missing = if matches!(scenario, Scenario::UndeclaredMissing) {
        vec![]
    } else {
        vec![
            PathBuf::from("project/missing.py"),
            PathBuf::from("project/unrequested.py"),
        ]
    };
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(
            vec![ryo::linter::RuleId::try_from("R1").unwrap()],
        ),
        rule_observations:
            ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots:
            ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode: if matches!(scenario, Scenario::WrongMode) {
            ProjectContextMode::Isolated
        } else {
            ProjectContextMode::ListedDisk
        },
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Sealed(
            ContextManifest {
                boundary: PathBuf::from("/snapshot"),
                logical_root: PathBuf::from("project"),
                auxiliary_files: BTreeMap::new(),
                source_files: BTreeMap::from([(
                    PathBuf::from("project/readable.py"),
                    ArtifactDigest("sealed".into()),
                )]),
                missing,
            },
        ),
        root: PathBuf::from("/snapshot/project"),
        rules: PathBuf::from("rules"),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration:
            ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language:
            ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![
            ryo::linter::RelativePath("readable.py".into()),
            ryo::linter::RelativePath("missing.py".into()),
        ],
        state: WorkerState::Fresh,
    };
    prepare(request)
        .map(|prepared| {
            let prepared = prepared.missing;
            assert!(
                prepared
                    .selected_missing
                    .contains(&ryo::linter::RelativePath("missing.py".into()))
            );
            assert!(
                !prepared
                    .selected_missing
                    .contains(&ryo::linter::RelativePath("readable.py".into()))
            );
            assert!(
                !prepared
                    .selected_missing
                    .contains(&ryo::linter::RelativePath("unrequested.py".into()))
            );
            prepared.selected_missing.len()
        })
        .map_err(|error| match error {
            ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message)
                if message == "context source seal differs from requested inventory" =>
            {
                "context source seal differs from requested inventory"
            }
            ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(message)
                if message == "worker context mode requires a matching explicit manifest" =>
            {
                "worker context mode requires a matching explicit manifest"
            }
            ryo_benchmark_contracts::domain::reports::errors::ReportsError::Invalid(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Io(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Json(_)
            | ryo_benchmark_contracts::domain::reports::errors::ReportsError::Toml(_) => {
                "unexpected failure"
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
