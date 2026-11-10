//! Authored project cases reach the SDK through the real suite and prepared worker boundaries.
use super::constants::{
    ALWAYS, BENCHMARK_TOML, BROKEN_OFFSET, BROKEN_SOURCE, CASES, CASES_TOML,
    EXPLICIT_PYTHON_UNRELATED_TOML_STI, FULL, INVALID_SYNTAX, INVALID_TOML_AND_INVALID_PYTHON,
    KEY_CASES, KEY_COMPARISON, KEY_CONTEXT, KEY_END, KEY_EXPECTED, KEY_FILE, KEY_FILES,
    KEY_LOGICAL_ROOT, KEY_NAME, KEY_RULE, KEY_START, OTHER_TOML,
    OTHER_TOML_PYTHON_SOURCE_WITHOUT_M, PROFILE, PROFILES_RUF200_BENCHMARK_TOML, PROJECT, PYTHON,
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
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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
    cases_text.push('\n');
    cases_text.push_str(&explicit_python_case()?);
    std::fs::write(&cases_path, cases_text)?;
    let cases =
        ryo_benchmark::inbound::use_cases::prepare_cases::use_case::PrepareCasesUseCaseImpl::new(
            &suite::local(),
            &ryo_benchmark::outbound::context::factory::local(),
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
            selected_rules: ryo_benchmark_contracts::domain::requests::types::CohortSelection::Explicit(vec![ryo::linter::RuleId::try_from(RUF200).unwrap()]),
            rule_observations: ryo_benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
            rule_context: ryo_benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
            project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: ProjectContextMode::ListedDisk,
            context_manifest: case.request.context_manifest.clone(),
            root: case.request.root.clone(),
            rules: rules.clone(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::from([(ryo_benchmark_contracts::domain::requests::types::RuleLabel(RUF200.into()), ryo_benchmark_contracts::domain::requests::types::ApplicabilityPolicy(ALWAYS.into()))]),
            parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(ryo_benchmark_contracts::domain::requests::types::ParserConfigurationName(PYTHON_TARGET_3_10.into())),
            source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Selected(ryo_benchmark_contracts::domain::requests::types::SourceLanguageName(PYTHON.into())),
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

/// The case file text of the one case that reads Python out of a file named for TOML, written by the TOML serializer.
fn explicit_python_case() -> Result<String, Error> {
    let text = |value: &str| toml::Value::String(value.to_owned());
    let table = |entries: Vec<(&str, toml::Value)>| {
        toml::Value::Table(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    };
    let finding = table(vec![
        (KEY_RULE, text(INVALID_SYNTAX)),
        (KEY_FILE, text(OTHER_TOML)),
        (KEY_START, toml::Value::Integer(BROKEN_OFFSET)),
        (KEY_END, toml::Value::Integer(BROKEN_OFFSET)),
    ]);
    let case = table(vec![
        (KEY_NAME, text(EXPLICIT_PYTHON_UNRELATED_TOML_STI)),
        (KEY_COMPARISON, text(FULL)),
        (KEY_FILES, table(vec![(OTHER_TOML, text(BROKEN_SOURCE))])),
        (KEY_CONTEXT, table(vec![(KEY_LOGICAL_ROOT, text(PROJECT))])),
        (KEY_EXPECTED, toml::Value::Array(vec![finding])),
    ]);
    let document = table(vec![(KEY_CASES, toml::Value::Array(vec![case]))]);
    toml::to_string(&document).map_err(|error| Error::Invalid(error.to_string()))
}
