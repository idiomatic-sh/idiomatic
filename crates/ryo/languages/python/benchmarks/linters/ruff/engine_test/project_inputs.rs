//! Authored project cases reach the SDK through the real suite and prepared worker boundaries.
use super::*;
use ryo_benchmark::outbound::suite::{factory as suite, types::ArtifactPath};
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::requests::types::WorkerState;
use std::collections::BTreeSet;

#[test]
fn listed_project_inputs_and_invalid_sources_keep_distinct_readers() -> Result<(), Error> {
    let scratch = tempfile::tempdir()?;
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff");
    let profile = scratch.path().join("profile");
    let rules = profile.join("rules");
    let rule = rules.join("RUF200");
    std::fs::create_dir_all(&rule)?;
    let _profile = std::fs::copy(
        source.join("profiles/RUF200/benchmark.toml"),
        profile.join("benchmark.toml"),
    )?;
    for name in ["query.ryoql", "cases.toml"] {
        let _source = std::fs::copy(source.join("rules/RUF200").join(name), rule.join(name))?;
    }
    let cases_path = rule.join("cases.toml");
    let mut cases_text = std::fs::read_to_string(&cases_path)?;
    cases_text.push_str(
        "\n[[cases]]\nname = \"explicit-python-unrelated-toml-still-parses\"\ncomparison = \"full\"\nfiles = { \"other.toml\" = \"def broken(\\n\" }\ncontext = { logical-root = \"project\" }\nexpected = [{ rule = \"invalid-syntax\", file = \"other.toml\", start = 12, end = 12 }]\n",
    );
    std::fs::write(&cases_path, cases_text)?;
    let cases = suite::local().cases(
        ArtifactPath(&rules),
        ArtifactPath(&scratch.path().join("cases")),
    )?;
    let selected = [
        "invalid-toml-and-invalid-python",
        "other-toml-python-source-without-manifest-diagnostic",
        "unknown-extension-still-python",
        "explicit-python-unrelated-toml-still-parses",
    ];
    let mut seen = BTreeSet::new();
    let engine = factory::local_with_frontend_diagnostic(
        python_310(),
        None,
        frontend_projection(EngineScenario::Finding)?,
    );
    for case in &cases {
        if !selected.contains(&case.name.0.as_ref()) {
            continue;
        }
        let _new = seen.insert(case.name.0.as_ref());
        let report = engine.run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(vec![ryo::linter::RuleId::try_from("RUF200").unwrap()]),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode: ProjectContextMode::ListedDisk,
            context_manifest: case.request.context_manifest.clone(),
            root: case.request.root.clone(),
            rules: rules.clone(),
            query_modules: BTreeMap::new(),
            rule_applicability: BTreeMap::from([("RUF200".into(), "always".into())]),
            parser_configuration: Some("python-target=3.10".into()),
            source_language: Some("python".into()),
            files: case.request.files.clone(),
            state: WorkerState::Fresh,
        })?;
        ryo_benchmark::domain::diagnostics::service::require_complete_execution(&report)
            .map_err(|message| Error::Invalid(message.into()))?;
        ryo_benchmark::domain::diagnostics::service::compare_diagnostics(&case.expected, &report)
            .map_err(|difference| Error::Invalid(format!("{}: {difference:?}", case.name.0)))?;
        assert_eq!(report.files, case.request.files);
    }
    assert_eq!(seen, selected.into_iter().collect());
    Ok(())
}
