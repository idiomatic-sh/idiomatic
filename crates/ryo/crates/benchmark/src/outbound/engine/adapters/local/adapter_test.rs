//! The concrete local implementation executes the complete shared contract.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::contract::check;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&crate::outbound::engine::factory::local(
        ryo::linter::Languages::default(),
        None,
    ));
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn actual_sdk_executes_missing_input_facts_but_refuses_unmodeled_provider_unread() {
    use crate::domain::suite::types::{
        CaseContext, ProjectContextMode, WorkerRequest, WorkerState,
    };
    use crate::outbound::engine::interface::EnginePort as _;
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
    let report = engine.run(request.clone()).unwrap();
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
    assert!(
        engine
            .run(readable)
            .unwrap_err()
            .to_string()
            .contains("incomplete candidate"),
        "a readable source refused by all providers is not a declared input absence"
    );
}

#[cfg(unix)]
#[test]
fn frontend_scope_uses_literal_missing_paths_not_glob_patterns() {
    use crate::domain::suite::types::{
        ContextManifest, ProjectContextMode, WorkerRequest, WorkerState,
    };
    let missing = r"pkg/missing[1]*?{x}é\name.py";
    let readable = r"pkg/missing1anythingQxé\name.py";
    let request = WorkerRequest {
        project_context_mode: ProjectContextMode::ListedDisk,
        context_manifest: Some(ContextManifest {
            boundary: "/snapshot".into(),
            logical_root: "project".into(),
            source_files: Default::default(),
            auxiliary_files: Default::default(),
            missing: vec![
                format!("project/{missing}").into(),
                "project/unrequested.py".into(),
            ],
        }),
        root: "/snapshot/project".into(),
        rules: "rules".into(),
        query_modules: Default::default(),
        rule_applicability: Default::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![
            ryo::linter::RelativePath(missing.into()),
            ryo::linter::RelativePath(readable.into()),
        ],
        state: WorkerState::Fresh,
    };
    let exclusions = super::adapter::frontend_exclusions(&request).unwrap();
    assert_eq!(
        exclusions.len(),
        1,
        "neither readable nor unrequested paths are exempted"
    );
    let matcher = globset::GlobBuilder::new(exclusions[0].as_ref())
        .literal_separator(true)
        .build()
        .unwrap()
        .compile_matcher();
    assert!(matcher.is_match(missing));
    assert!(!matcher.is_match(readable));
    assert!(!matcher.is_match("pkg/missing1anythingQxéname.py"));
}
