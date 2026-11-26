//! A syntax-rejected file beside a valid one: the rules answer the valid file as Ruff does. Per-file partitions and skip
//! accounting are proved by core.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::default_frontend_projection;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::python_310;
use benchmark::outbound::engine::factory;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

/// A syntax-rejected file beside a valid one, as Django ships one: the pytest decorator rules PT020, PT025 and PT026 skip the
/// rejected file and answer the valid one with Ruff 0.16.5's findings. While their joins on a byte offset were not keyed by file,
/// their plans ran across files and the one rejected file left each of them Unknown at every file.
pub(super) fn a_rejected_file_leaves_pt020_pt025_and_pt026_answering_the_valid_file()
-> Result<(), Error> {
    use benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    super::rule_fixtures_test::write_suppress_module(root.path())?;
    // The three rules find a decorator's first expression token through these fragments.
    let mut modules = super::prepared_engine_test::suppress_modules();
    for (name, source) in [
        (
            "decorator-expression-starts",
            include_str!("../fragments/decorator-expression-starts.ryoql"),
        ),
        ("next-token", include_str!("../fragments/next-token.ryoql")),
        (
            "code-tokens",
            include_str!("../fragments/code-tokens.ryoql"),
        ),
    ] {
        let file = format!("fragments/{name}.ryoql");
        std::fs::write(root.path().join(&file), source)?;
        let _previous = modules.insert(format!("python/ruff/{name}"), file.into());
    }
    for (rule, query) in [
        ("PT020", include_str!("../rules/PT020/query.ryoql")),
        ("PT025", include_str!("../rules/PT025/query.ryoql")),
        ("PT026", include_str!("../rules/PT026/query.ryoql")),
    ] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    std::fs::write(
        root.path().join("case.py"),
        "import pytest\n@pytest.yield_fixture\ndef resource():\n    yield 1\n@pytest.mark.usefixtures(\"resource\")\n@pytest.fixture\ndef combined():\n    return 1\n@pytest.mark.usefixtures()\ndef test_case():\n    pass\n",
    )?;
    std::fs::write(root.path().join("broken.py"), "def broken(:\n    pass\n")?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: benchmark_contracts::domain::requests::types::CohortSelection::Explicit(fixture_cohort(&["PT020", "PT025", "PT026"])),
        rule_observations: benchmark_contracts::domain::requests::types::InputDeclaration::Undeclared,
        rule_context: benchmark_contracts::domain::requests::types::RuleContextPreparation::Unconfigured,
        project_search_roots: benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        given_project_inventory: Default::default(),
        project_context_mode:
            benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: benchmark_contracts::domain::requests::types::ContextSeal::Unsealed,
        root: root.path().to_owned(),
        rules,
        query_modules: super::prepared_engine_test::module_names(modules),
        rule_applicability: BTreeMap::from([
            (benchmark_contracts::domain::requests::types::RuleLabel("PT020".to_owned()), benchmark_contracts::domain::requests::types::ApplicabilityPolicy("accepted-syntax".to_owned())),
            (benchmark_contracts::domain::requests::types::RuleLabel("PT025".to_owned()), benchmark_contracts::domain::requests::types::ApplicabilityPolicy("accepted-syntax".to_owned())),
            (benchmark_contracts::domain::requests::types::RuleLabel("PT026".to_owned()), benchmark_contracts::domain::requests::types::ApplicabilityPolicy("accepted-syntax".to_owned())),
        ]),
        parser_configuration: benchmark_contracts::domain::requests::types::ParserSelection::Named(benchmark_contracts::domain::requests::types::ParserConfigurationName(
            "python-target=3.10".to_owned(),
        )),
        source_language: benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        files: vec![
            ryo::linter::RelativePath("broken.py".into()),
            ryo::linter::RelativePath("case.py".into()),
        ],
        state: benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let report =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?)
            .run_prepared(request)?;
    let finding =
        |rule: &str, file: &str, start: usize, end: usize| -> Result<BenchmarkFinding, Error> {
            Ok(BenchmarkFinding {
                rule: ryo::linter::RuleId::try_from(rule)
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                file: ryo::linter::RelativePath(file.into()),
                start: SourceByteOffset(start),
                end: SourceByteOffset(end),
            })
        };
    let mut expected = vec![
        finding("invalid-syntax", "broken.py", 11, 12)?,
        finding("invalid-syntax", "broken.py", 12, 13)?,
        finding("PT020", "case.py", 14, 35)?,
        finding("PT025", "case.py", 64, 100)?,
        finding("PT026", "case.py", 146, 172)?,
    ];
    expected.sort();
    let mut actual = report.diagnostics;
    actual.sort();
    assert_eq!(
        actual, expected,
        "each rule answers the valid file as Ruff 0.16.5 does"
    );
    assert_eq!(
        report.skipped.len(),
        3,
        "the rejected file is skipped by each rule, not Unknown"
    );
    Ok(())
}
