//! Literal properties and decoded numeric payloads reach engine queries.
use super::prepared_engine_test::PreparedEngine as _;
use super::prepared_engine_test::fixture_cohort;
use super::prepared_engine_test::python_310;
use ryo_benchmark::outbound::engine::factory;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::WorkerRequest;
use std::collections::BTreeMap;

pub(super) fn literal_properties_reach_engine_queries_for_true_and_false_boolean_nodes()
-> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    let rule_directory = rules.join("LIT");
    std::fs::create_dir_all(&rule_directory)?;
    std::fs::write(
        rule_directory.join("query.ryoql"),
        "let truth = select id, file, span from \"literal-properties\" where kind = \"boolean\" and boolean = \"true\";\nemit truth subject file key(span);",
    )?;
    std::fs::write(root.path().join("true.py"), "value = True\n")?;
    std::fs::write(root.path().join("false.py"), "value = False\n")?;
    let report = factory::local(python_310(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["LIT"])),
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
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: ["true.py", "false.py"]
            .into_iter()
            .map(|path| ryo::linter::RelativePath(path.into()))
            .collect(),
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    assert_eq!(report.files.len(), 2, "both files must be completely read");
    assert!(
        report
            .files
            .contains(&ryo::linter::RelativePath("true.py".into()))
    );
    assert!(
        report
            .files
            .contains(&ryo::linter::RelativePath("false.py".into()))
    );
    assert_eq!(report.diagnostics.len(), 1);
    let finding = report
        .diagnostics
        .first()
        .ok_or_else(|| Error::Invalid("the true literal should produce a finding".to_owned()))?;
    assert_eq!(finding.rule.as_ref(), "LIT");
    assert_eq!(finding.file.0.as_ref(), "true.py");
    assert_eq!(finding.start.0, 8);
    assert_eq!(finding.end.0, 12);
    Ok(())
}

pub(super) fn decoded_numeric_payloads_and_strict_greater_composition_reach_engine_queries()
-> Result<(), Error> {
    let root = tempfile::tempdir()?;
    let rules = root.path().join("rules");
    for (rule, query) in [
        (
            "EXACT",
            "let exact = select id, file, span from \"numeric-literal\" where kind = \"integer\" and numeric_gte(value, \"9007199254740993\"); emit exact subject file key(id);",
        ),
        (
            "STRICT",
            "let above = select id, file, span from \"numeric-literal\" where numeric_gte(value, 86400); let below = select id, file from \"numeric-literal\" where numeric_gte(86400, value); let strict = antijoin(left=above, right=below, left-on=[\"id\", \"file\"], right-on=[\"id\", \"file\"]); emit strict subject file key(id);",
        ),
        (
            "SUCCESSOR",
            "let adjacent = select id, file, span from \"numeric-literal\" where integer_successor(value, 9007199254740993); emit adjacent subject file key(id);",
        ),
    ] {
        let directory = rules.join(rule);
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("query.ryoql"), query)?;
    }
    let source = "0x10\n1_000\n9007199254740992\n9007199254740993\n86400.0\n86400.25\n86401.0\n1e9999\nTrue\n1j\n";
    std::fs::write(root.path().join("numeric.py"), source)?;
    let report = factory::local(python_310(), None).run_prepared(WorkerRequest {
        performance: Default::default(),
        selected_rules: Some(fixture_cohort(&["EXACT", "STRICT", "SUCCESSOR"])),
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
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        files: vec![ryo::linter::RelativePath("numeric.py".into())],
        state: ryo_benchmark_contracts::domain::requests::types::WorkerState::Fresh,
    })?;
    assert_eq!(
        report.files,
        [ryo::linter::RelativePath("numeric.py".into())]
    );
    assert!(report.skipped.is_empty());
    let mut actual = report
        .diagnostics
        .iter()
        .map(|finding| (finding.rule.as_ref(), finding.start.0, finding.end.0))
        .collect::<Vec<_>>();
    let mut expected = Vec::new();
    for (rule, text) in [
        ("EXACT", "9007199254740993"),
        ("SUCCESSOR", "9007199254740992"),
        ("STRICT", "9007199254740992"),
        ("STRICT", "9007199254740993"),
        ("STRICT", "86400.25"),
        ("STRICT", "86401.0"),
        ("STRICT", "1e9999"),
    ] {
        let start = source
            .find(text)
            .expect("independent numeric fixture occurrence");
        expected.push((rule, start, start + text.len()));
    }
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
    Ok(())
}
