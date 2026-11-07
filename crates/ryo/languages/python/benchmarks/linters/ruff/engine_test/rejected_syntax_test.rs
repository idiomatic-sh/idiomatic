//! Rejected syntax with provider partials skips only accepted-syntax rules.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::default_frontend_projection;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::python_310;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn rejected_syntax_with_provider_partials_skips_only_accepted_syntax_rules()
-> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    let query =
        "let names = select id, file from \"qualified-name\";\nemit names subject file key(id);\n";
    for rule in ["ASYNC100", "UP025"] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    std::fs::write(root.path().join("input.py"), "value = ur\"hello\"\n")?;
    let request = WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["ASYNC100", "UP025"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::from([
            ("ASYNC100".to_owned(), "always".to_owned()),
            ("UP025".to_owned(), "accepted-syntax".to_owned()),
        ]),
        parser_configuration: Some("python-target=3.10".to_owned()),
        source_language: None,
        files: vec![ryo::linter::RelativePath("input.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    };
    let result =
        factory::local_with_frontend_diagnostic(python_310(), None, default_frontend_projection()?)
            .run_prepared(request);
    match result {
        Err(Error::Invalid(message)) => {
            assert!(
                message.contains("ASYNC100"),
                "always-applicable partial must remain unknown: {message}"
            );
            assert!(
                !message.contains("UP025"),
                "intentionally skipped rule became unknown: {message}"
            );
        }
        Err(error) => return Err(error),
        Ok(unexpected) => {
            drop(unexpected);
            return Err(Error::Invalid(
                "provider partial was not retained for the always rule".into(),
            ));
        }
    }
    Ok(())
}

/// A syntax-rejected file beside a valid one, as Django ships one: the pytest decorator rules PT020, PT025 and PT026 skip the
/// rejected file and answer the valid one with Ruff 0.16.5's findings. While their joins on a byte offset were not keyed by file,
/// their plans ran across files and the one rejected file left each of them Unknown at every file.
pub(super) fn a_rejected_file_leaves_pt020_pt025_and_pt026_answering_the_valid_file()
-> Result<(), Error> {
    use ryo_benchmark_contracts::domain::reports::types::{BenchmarkFinding, SourceByteOffset};
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    super::rule_fixtures_test::write_suppress_module(root.path())?;
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
        selected_rules: Some(fixture_cohort(&["PT020", "PT025", "PT026"])),
        rule_observations: None,
        rule_context: None,
        project_search_roots: None,
        given_project_inventory: Default::default(),
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        context_manifest: None,
        root: root.path().to_owned(),
        rules,
        query_modules: super::prepared_engine_test::suppress_modules(),
        rule_applicability: BTreeMap::from([
            ("PT020".to_owned(), "accepted-syntax".to_owned()),
            ("PT025".to_owned(), "accepted-syntax".to_owned()),
            ("PT026".to_owned(), "accepted-syntax".to_owned()),
        ]),
        parser_configuration: Some("python-target=3.10".to_owned()),
        source_language: None,
        files: vec![
            ryo::linter::RelativePath("broken.py".into()),
            ryo::linter::RelativePath("case.py".into()),
        ],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
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
