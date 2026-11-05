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

#[derive(Clone, Copy)]
enum Scenario {
    Sealed,
    UndeclaredMissing,
    WrongMode,
    ActualSdk,
}
type Input = Scenario;
type Value = usize;
type Failure = &'static str;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("only selected sealed absences are passed to the SDK adapter"),
        word: Word::Succeeds,
        input: || Scenario::Sealed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "actual SDK observes declared missing input without masking provider refusal",
        ),
        word: Word::Succeeds,
        input: || Scenario::ActualSdk,
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
    if matches!(scenario, Scenario::ActualSdk) {
        actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread();
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
        selected_rules: Some(vec![ryo::linter::RuleId::try_from("R1").unwrap()]),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode: if matches!(scenario, Scenario::WrongMode) {
            ProjectContextMode::Isolated
        } else {
            ProjectContextMode::ListedDisk
        },
        context_manifest: Some(ContextManifest {
            boundary: PathBuf::from("/snapshot"),
            logical_root: PathBuf::from("project"),
            auxiliary_files: BTreeMap::new(),
            source_files: BTreeMap::from([(
                PathBuf::from("project/readable.py"),
                ArtifactDigest("sealed".into()),
            )]),
            missing,
        }),
        root: PathBuf::from("/snapshot/project"),
        rules: PathBuf::from("rules"),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: None,
        source_language: None,
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
            _ => "unexpected failure",
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

fn actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread() {
    use crate::outbound::engine::interface::EnginePort as _;
    use ryo_benchmark_contracts::domain::context::types::CaseContext;
    use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
    use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
    use ryo_benchmark_contracts::domain::requests::types::WorkerState;
    let scratch = tempfile::tempdir().unwrap();
    let rules = scratch.path().join("rules");
    std::fs::create_dir_all(rules.join("R1")).unwrap();
    std::fs::write(rules.join("R1/query.ryoql"), "let failures = select path as file, span from \"filesystem.read-failure\"; emit failures subject file key(file);").unwrap();
    let boundary = scratch.path().join("snapshot");
    let manifest = crate::outbound::context::adapters::local::adapter::materialize(
        &boundary,
        &CaseContext {
            logical_root: "project".into(),
            auxiliary_files: Default::default(),
            missing: vec!["project/missing.py".into()],
        },
        &Default::default(),
    )
    .unwrap();
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(vec![ryo::linter::RuleId::try_from("R1").unwrap()]),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode: ProjectContextMode::ListedDisk,
        context_manifest: Some(manifest),
        root: boundary.join("project"),
        rules,
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("missing.py".into())],
        state: WorkerState::Fresh,
    };
    let engine = crate::outbound::engine::factory::local_with_frontend_diagnostic(
        ryo::linter::Languages::default(), None,
        crate::outbound::engine::factory::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from("P001").unwrap(),
            query: ryo::query::RyoQlQuery::try_from("let diagnostics = select id, file, span from \"syntax-error\"; emit diagnostics subject file key(id);").unwrap(),
        },
    );
    let prepared = prepare(request.clone()).unwrap();
    assert_eq!(
        prepared.missing.selected_missing,
        request.files.iter().cloned().collect()
    );
    let report = engine.run(prepared.request, prepared.missing).unwrap();
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].file, request.files[0]);
    assert_eq!(report.diagnostics[0].start.0, 0);
    assert_eq!(report.diagnostics[0].end.0, 0);
    let mut readable = request;
    let source = [(
        ryo::linter::RelativePath("input.txt".into()),
        ryo::linter::SourceText("value = 1\n".into()),
    )]
    .into();
    readable.context_manifest = Some(
        crate::outbound::context::adapters::local::adapter::materialize(
            &boundary,
            &CaseContext {
                logical_root: "project".into(),
                ..Default::default()
            },
            &source,
        )
        .unwrap(),
    );
    readable.files = vec![ryo::linter::RelativePath("input.txt".into())];
    let prepared = prepare(readable).unwrap();
    assert!(prepared.missing.selected_missing.is_empty());
    assert!(
        engine
            .run(prepared.request, prepared.missing)
            .unwrap_err()
            .to_string()
            .contains("incomplete candidate"),
        "a readable source refused by all providers is not a declared input absence"
    );
}
