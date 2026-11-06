//! Authored project cases reach the SDK through the real suite and prepared worker boundaries.
use super::constants::{
    ALWAYS, BENCHMARK_TOML, BENCHMARKS_LINTERS_RUFF, CASES, CASES_NAME_EXPLICIT_PYTHON_UNRELAT,
    CASES_TOML, EXPLICIT_PYTHON_UNRELATED_TOML_STI, INVALID_TOML_AND_INVALID_PYTHON,
    OTHER_TOML_PYTHON_SOURCE_WITHOUT_M, PROFILE, PROFILES_RUF200_BENCHMARK_TOML, PYTHON,
    PYTHON_TARGET_3_10, QUERY_RYOQL, RUF200, RULES, RULES_RUF200, UNKNOWN_EXTENSION_STILL_PYTHON,
};
use super::prepared_engine_test::{PreparedEngine as _, default_frontend_projection, python_310};
use ryo_benchmark::inbound::use_cases::prepare_cases::interface::PrepareCasesUseCase as _;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark::outbound::suite::factory as suite;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub(super) fn listed_project_inputs_and_invalid_sources_keep_distinct_readers() -> Result<(), Error>
{
    let scratch = tempfile::tempdir()?;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(BENCHMARKS_LINTERS_RUFF);
    let profile = scratch.path().join(PROFILE);
    let rules = profile.join(RULES);
    let rule = rules.join(RUF200);
    std::fs::create_dir_all(&rule)?;
    let _profile = std::fs::copy(
        source.join(PROFILES_RUF200_BENCHMARK_TOML),
        profile.join(BENCHMARK_TOML),
    )?;
    for name in [QUERY_RYOQL, CASES_TOML] {
        let _source = std::fs::copy(source.join(RULES_RUF200).join(name), rule.join(name))?;
    }
    let cases_path = rule.join(CASES_TOML);
    let mut cases_text = std::fs::read_to_string(&cases_path)?;
    cases_text.push_str(CASES_NAME_EXPLICIT_PYTHON_UNRELAT);
    std::fs::write(&cases_path, cases_text)?;
    let cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite::local(),
        )
        .prepare_cases(&rules, &scratch.path().join(CASES))?;
    let selected = [
        INVALID_TOML_AND_INVALID_PYTHON,
        OTHER_TOML_PYTHON_SOURCE_WITHOUT_M,
        UNKNOWN_EXTENSION_STILL_PYTHON,
        EXPLICIT_PYTHON_UNRELATED_TOML_STI,
    ];
    let mut seen = BTreeSet::new();
    let engine =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?);
    for case in &cases {
        if !selected.contains(&case.name.0.as_ref()) {
            continue;
        }
        let _new = seen.insert(case.name.0.as_ref());
        let report = engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(vec![ryo::linter::RuleId::try_from(RUF200).unwrap()]),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode: ProjectContextMode::ListedDisk,
            context_manifest: case.request.context_manifest.clone(),
            root: case.request.root.clone(),
            rules: rules.clone(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::from([(RUF200.into(), ALWAYS.into())]),
            parser_configuration: Some(PYTHON_TARGET_3_10.into()),
            source_language: Some(PYTHON.into()),
            files: case.request.files.clone(),
            state: WorkerState::Fresh,
        })?;
        ryo_benchmark_contracts::domain::reports::service::require_complete_execution(&report)?;
        ryo_benchmark::domain::diagnostics::service::compare_diagnostics(&case.expected, &report)
            .map_err(|difference| Error::Invalid(format!("{}: {difference:?}", case.name.0)))?;
        assert_eq!(report.files, case.request.files);
    }
    assert_eq!(seen, selected.into_iter().collect());
    Ok(())
}
