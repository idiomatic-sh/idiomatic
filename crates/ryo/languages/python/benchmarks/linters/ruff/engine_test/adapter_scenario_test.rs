//! The scenarios of the Ruff adapter proofs and the invocations they expect.
use super::super::adapter::LinterRuffAdapter;
use super::super::settings::LinterRuffSettings;
use ryo::linter::RuleId;
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

#[derive(Clone, Copy)]
pub(super) enum AdapterScenario {
    Contract,
    Operational,
    EmptyInput,
    Invocation,
    InvocationPy311,
    MissingExecutable,
    WarningFinding,
    MissingCleanFile,
    CleanWarning,
    InvalidJson,
    ContradictoryStatus,
    UnrequestedFile,
    DuplicateInput,
    UnexpectedRule,
    ReversedSpan,
    CanonicalRoot,
    FrontendFinding,
    ReferenceRootsMustMatchTheDeclaredProjectScope,
}

#[expect(clippy::expect_used, reason = "the literal fixture rule is valid")]
pub(super) fn reference_roots_must_match_the_declared_project_scope() -> Result<(), Error> {
    use ryo::linter::RelativePath;
    let adapter = LinterRuffAdapter::new(LinterRuffSettings {
        executable: "/missing-ruff".into(),
        defaults: [RuleId::try_from("I001").expect("valid rule")]
            .into_iter()
            .collect(),
        target_version: "py310".into(),
    });
    let mut request = RunRequest {
        performance: Default::default(),
        selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(vec![RelativePath("".into()), RelativePath("src".into())]),
        given_project_inventory: Default::default(),
        staged_sources: ryo_benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata: ryo_benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: ryo_benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "/tmp".into(),
        files: vec![RelativePath("input.py".into())],
        state: "/tmp/cache".into(),
    };
    adapter.prepare(&request)?;
    request.project_search_roots = ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(vec![RelativePath("".into())]);
    assert!(matches!(adapter.prepare(&request), Err(Error::Invalid(_))));
    request.project_search_roots = ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(Vec::new());
    assert!(matches!(adapter.prepare(&request), Err(Error::Invalid(_))));
    request.project_search_roots = ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared;
    adapter.prepare(&request)?;
    let unrelated = LinterRuffAdapter::new(LinterRuffSettings {
        executable: "/missing-ruff".into(),
        defaults: [RuleId::try_from("F401").expect("valid rule")]
            .into_iter()
            .collect(),
        target_version: "py310".into(),
    });
    request.project_search_roots = ryo_benchmark_contracts::domain::requests::types::SearchRoots::Declared(vec![RelativePath("".into())]);
    assert!(matches!(
        unrelated.prepare(&request),
        Err(Error::Invalid(_))
    ));
    Ok(())
}

pub(super) fn invocation_matches(
    ruff: &LinterRuffAdapter,
    request: &RunRequest,
    executable: &std::path::Path,
    target_version: &str,
) -> Result<bool, Error> {
    let invocation = ruff.prepare(request)?;
    let expected: Vec<std::ffi::OsString> = [
        "check",
        "--isolated",
        "--output-format",
        "json",
        "--cache-dir",
    ]
    .into_iter()
    .map(Into::into)
    .chain([
        request.state.join("ruff").into_os_string(),
        "--select".into(),
        "E722".into(),
        "--target-version".into(),
        target_version.into(),
        "--".into(),
        "case.py".into(),
    ])
    .collect();
    Ok(invocation.arguments == expected
        && invocation
            .environment
            .get(std::ffi::OsStr::new(
                ryo_benchmark_contracts::domain::measurements::constants::THREAD_PIN,
            ))
            == Some(&"1".into())
        && invocation.directory == request.root
        && invocation.executable == executable)
}

/// Build independent reference-wire coordinates for valid and deliberately invalid cases.
pub(super) fn finding_bytes(
    scenario: AdapterScenario,
    filename: &std::path::Path,
) -> Result<Vec<u8>, Error> {
    let code = if matches!(scenario, AdapterScenario::UnexpectedRule) {
        "F401"
    } else {
        "E722"
    };
    let end = if matches!(scenario, AdapterScenario::ReversedSpan) {
        1_usize
    } else {
        7_usize
    };
    let start = if matches!(scenario, AdapterScenario::ReversedSpan) {
        7_usize
    } else {
        1_usize
    };
    let mut findings = vec![serde_json::json!({
        "code": code,
        "filename": filename,
        "cell": null,
        "location": {"row": 3_usize, "column": start},
        "end_location": {"row": 3_usize, "column": end}
    })];
    if matches!(scenario, AdapterScenario::FrontendFinding) {
        findings.push(serde_json::json!({
            "code": "invalid-syntax",
            "filename": filename,
            "cell": null,
            "location": {"row": 3_usize, "column": 7_usize},
            "end_location": {"row": 3_usize, "column": 8_usize}
        }));
    }
    serde_json::to_vec(&findings).map_err(|error| Error::Json(Box::new(error)))
}
