//! A changed projection, query module or target configuration invalidates exactly what it should.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::python_310;
use super::prepared_engine_test::suppress_modules;
use super::rule_fixtures_test::{write_rule_fixtures, write_suppress_module};
use super::scenario_expectations_test::{input_file, source_for};
use super::scenario_test::EngineScenario;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::BenchmarkFinding;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn projection_cache_query_change(root: &std::path::Path) -> Result<bool, Error> {
    let rules = root.join("rules");
    let selected_rules = write_rule_fixtures(&rules, EngineScenario::ProjectionCacheQueryChange)?;
    let file = input_file(EngineScenario::ProjectionCacheQueryChange);
    std::fs::write(
        root.join(file),
        source_for(EngineScenario::ProjectionCacheQueryChange),
    )?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(selected_rules),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.to_owned(),
        rules,
        query_modules: suppress_modules(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath(file.into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let rule = ryo::linter::RuleId::try_from("cache-probe")
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let run = |code| {
        let query = format!(
            "let diagnostics = select id, file, span from \"syntax-error\" where code = \"{code}\";\nemit diagnostics subject file key(id);\n"
        );
        let projection = factory::FrontendDiagnosticProjection {
            rule: rule.clone(),
            query: ryo::query::RyoQlQuery::try_from(query.as_str())
                .map_err(|error| Error::Invalid(error.to_string()))?,
        };
        Ok::<_, Error>(factory::local_with_frontend_diagnostic(
            python_310(),
            None,
            projection,
        ))
    };
    let first = run("nonlocal-at-module")?.run_prepared(WorkerRequest {
        project_search_roots: None,
        given_project_inventory: Default::default(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            root.join("retained-parse-cache"),
        ),
        ..request.clone()
    })?;
    let second = run("no-such-parser-code")?.run_prepared(WorkerRequest {
        project_search_roots: None,
        given_project_inventory: Default::default(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Retained(
            root.join("retained-parse-cache"),
        ),
        ..request
    })?;
    Ok(first.diagnostics.len() == 1
        && first
            .diagnostics
            .first()
            .is_some_and(|finding| finding.rule.as_ref() == "cache-probe")
        && second.diagnostics.is_empty())
}

pub(super) fn query_module_inline_equivalence(root: &std::path::Path) -> Result<bool, Error> {
    let rules = root.join("rules");
    let rule_dir = rules.join("fixture");
    std::fs::create_dir_all(&rule_dir)?;
    std::fs::write(root.join("input.py"), "value = 1\n")?;
    let input = "let nodes = select id, file, span from \"syntax-tree\";\n";
    let inline = format!("{input}emit nodes subject file key(span);\n");
    std::fs::write(rule_dir.join("query.ryoql"), inline)?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked()]);
    let plain = factory::local(languages.clone(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["fixture"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.into(),
        rules: rules.clone(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;

    let fragments = root.join("fragments");
    std::fs::create_dir_all(&fragments)?;
    std::fs::write(
        fragments.join("identity.ryoql"),
        "input source: relation<id: node, file: file, span: span>;\noutput relation<id: node, file: file, span: span>;\nlet rows = select id, file, span from source;\nreturn rows;\n",
    )?;
    std::fs::write(
        rule_dir.join("query.ryoql"),
        format!(
            "import identity from \"test/identity\";\n{input}let rows = identity(source=nodes);\nemit rows subject file key(span);\n"
        ),
    )?;
    let mut modules = BTreeMap::new();
    let _previous = modules.insert(
        "test/identity".to_owned(),
        std::path::PathBuf::from("fragments/identity.ryoql"),
    );
    let imported = factory::local(languages, None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["fixture"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.into(),
        rules,
        query_modules: modules,
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    Ok(plain == imported)
}

/// Stage every canonical source; its declared expectations are bound to Python3.11.
/// The same complete source set must be negative on3.10, before and after cache reuse.
pub(super) fn stage_up017_target_files(
    root: &std::path::Path,
) -> Result<(Vec<ryo::linter::RelativePath>, Vec<BenchmarkFinding>), Error> {
    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        name: String,
        files: BTreeMap<String, String>,
        expected: Vec<Finding>,
    }
    #[derive(serde::Deserialize)]
    struct Finding {
        rule: String,
        file: String,
        start: usize,
        end: usize,
    }
    let cases: Cases = toml::from_str(include_str!("../rules/UP017/cases.toml"))
        .map_err(|error| Error::Invalid(error.to_string()))?;
    if cases.cases.len() != 14 {
        return Err(Error::Invalid(
            "UP017 target control must retain all14 canonical cases".into(),
        ));
    }
    let (mut files, mut expected_py311) = (Vec::new(), Vec::new());
    for case in cases.cases {
        for finding in case.expected {
            if finding.rule != "UP017" || !case.files.contains_key(&finding.file) {
                return Err(Error::Invalid(format!(
                    "invalid UP017 py311 expectation in {}",
                    case.name
                )));
            }
            expected_py311.push(BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from(finding.rule.as_str())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(
                    format!("up017-target/{}/{}", case.name, finding.file).into(),
                ),
                start: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(
                    finding.start,
                ),
                end: ryo_benchmark_contracts::domain::reports::types::SourceByteOffset(finding.end),
            });
        }
        for (relative, text) in case.files {
            let path = format!("up017-target/{}/{}", case.name, relative);
            let destination = root.join(&path);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(destination, text)?;
            files.push(ryo::linter::RelativePath(path.into()));
        }
    }
    files.sort();
    expected_py311.sort();
    if files.len() != 14 || expected_py311.is_empty() {
        return Err(Error::Invalid(
            "UP017 target guard needs all14 sources and live py311 positives".into(),
        ));
    }
    Ok((files, expected_py311))
}

pub(super) fn target_configuration_cache_change(root: &std::path::Path) -> Result<bool, Error> {
    use ryo_benchmark_contracts::domain::requests::types::WorkerState;
    let rules = root.join("rules");
    let rule_directory = rules.join("UP017");
    std::fs::create_dir_all(&rule_directory)?;
    std::fs::write(
        rule_directory.join("query.ryoql"),
        include_str!("../rules/UP017/query.ryoql"),
    )?;
    write_suppress_module(root)?;
    let (files, expected_py311) = stage_up017_target_files(root)?;
    let cache = root.join("retained-target-cache");
    let query_modules = suppress_modules();
    let run = |target, identity: &'static str, state| {
        factory::local(
            ryo::linter::Languages(vec![ryo_python::linked_with_settings(
                ryo_python::settings::PythonParserSettings::new(target),
            )]),
            None,
        )
        .run_prepared(WorkerRequest {
            performance: Default::default(),
            selected_rules: Some(fixture_cohort(&["UP017"])),
            rule_observations: None,
            rule_context: None,
            project_search_roots: None,
            given_project_inventory: Default::default(),
            project_context_mode:
                ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
            context_manifest: None,
            root: root.to_owned(),
            rules: rules.clone(),
            query_modules: query_modules.clone(),
            rule_applicability: BTreeMap::default(),
            parser_configuration: Some(identity.to_owned()),
            source_language: None,
            files: files.clone(),
            state,
        })
    };
    for (target, identity, state, expected) in [
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Fresh,
            Vec::new(),
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Retained(cache.clone()),
            Vec::new(),
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py311,
            "python-target=3.11",
            WorkerState::Retained(cache.clone()),
            expected_py311,
        ),
        (
            ryo_python::settings::PythonTargetVersion::Py310,
            "python-target=3.10",
            WorkerState::Retained(cache.clone()),
            Vec::new(),
        ),
    ] {
        let report = run(target, identity, state)?;
        let mut observed_files = report.files;
        observed_files.sort();
        let mut observed = report.diagnostics;
        observed.sort(); // Canonical diagnostic multiset: preserve every duplicate.
        if observed_files != files || observed != expected {
            return Err(Error::Invalid(format!(
                "UP017 full14-source target/cache guard differs at {identity}: files={observed_files:?}; expected={expected:?}; observed={observed:?}"
            )));
        }
    }
    Ok(true)
}
