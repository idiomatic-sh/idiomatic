//! The scenarios of the Ruff adapter proofs and the invocations they expect.
use super::super::adapter::LinterRuffAdapter;
use super::super::settings::LinterRuffSettings;
use benchmark::outbound::linter::interface::LinterPort as _;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::RunRequest;
use ryo::linter::RuleId;

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
        selected_rules: benchmark_contracts::domain::requests::types::CohortSelection::Frozen,
        rule_observations:
            benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context:
            benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: benchmark_contracts::domain::requests::types::SearchRoots::Declared(
            vec![RelativePath("".into()), RelativePath("src".into())],
        ),
        given_project_inventory: Default::default(),
        staged_sources: benchmark_contracts::domain::requests::types::StagedSources::Unlisted,
        file_metadata:
            benchmark_contracts::domain::requests::types::FileMetadataDeclaration::Undeclared,
        context_manifest: benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: "/tmp".into(),
        files: vec![RelativePath("input.py".into())],
        state: "/tmp/cache".into(),
    };
    let _invocation = adapter.prepare(&request)?;
    request.project_search_roots =
        benchmark_contracts::domain::requests::types::SearchRoots::Declared(vec![RelativePath(
            "".into(),
        )]);
    assert!(matches!(adapter.prepare(&request), Err(Error::Invalid(_))));
    request.project_search_roots =
        benchmark_contracts::domain::requests::types::SearchRoots::Declared(Vec::new());
    assert!(matches!(adapter.prepare(&request), Err(Error::Invalid(_))));
    request.project_search_roots =
        benchmark_contracts::domain::requests::types::SearchRoots::Undeclared;
    let _invocation = adapter.prepare(&request)?;
    let unrelated = LinterRuffAdapter::new(LinterRuffSettings {
        executable: "/missing-ruff".into(),
        defaults: [RuleId::try_from("F401").expect("valid rule")]
            .into_iter()
            .collect(),
        target_version: "py310".into(),
    });
    request.project_search_roots =
        benchmark_contracts::domain::requests::types::SearchRoots::Declared(vec![RelativePath(
            "".into(),
        )]);
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
        && invocation.environment.get(std::ffi::OsStr::new(
            benchmark_contracts::domain::measurements::constants::THREAD_PIN,
        )) == Some(
            &benchmark_contracts::domain::measurements::constants::MEASURED_WORKERS
                .to_string()
                .into(),
        )
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
    // Only the temporary path is supplied by the environment. Quote that field
    // as a native process would. Keep the captured record independent of the decoder.
    let filename = crate::json_output::bytes(&filename).map_err(|e| Error::Json(Box::new(e)))?;
    let filename = std::str::from_utf8(&filename).expect("UTF-8 encoded fixture path");
    let record = |code, start, end| {
        format!(
            r#"{{"code":"{code}","filename":{filename},"cell":null,"location":{{"row":3,"column":{start}}},"end_location":{{"row":3,"column":{end}}}}}"#
        )
    };
    let mut stdout = format!("[{}", record(code, start, end));
    if matches!(scenario, AdapterScenario::FrontendFinding) {
        stdout.push(',');
        stdout.push_str(&record("invalid-syntax", 7, 8));
    }
    stdout.push(']');
    Ok(stdout.into_bytes())
}
