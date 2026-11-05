//! Actual decoded artifacts cannot hide missing defaults or certify negative-only evidence.
use super::service;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::suite::{factory::local, interface::SuitePort as _, types::ArtifactPath};
fn negative_only_evidence(directory: &std::path::Path) {
    let oracle = directory.join("oracle");
    std::fs::create_dir_all(oracle.join("rules/F621")).expect("oracle rule");
    std::fs::write(
        oracle.join("rules/F621/cases.toml"),
        r#"[[cases]]
name = "upstream inactive"
files = { "input.py" = "a = 1" }
expected = []
"#,
    )
    .expect("oracle cases");
    let tool = ryo_benchmark_contracts::domain::workloads::types::Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::workloads::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: None,
        file_observation: None,
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec!["pin".into()],
        defaults: vec!["F621".try_into().expect("rule identifier")],
        deferred: [(
            "F621".try_into().expect("rule identifier"),
            "upstream_inactive".into(),
        )]
        .into(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
    };
    let audited = service::audit(&local(), &oracle, &tool).expect("negative-only oracle evidence");
    assert!(audited.implemented.is_empty());
    assert_eq!(
        audited
            .missing
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["F621"]
    );
    std::fs::write(oracle.join("rules/F621/query.ryoql"), "nonempty query")
        .expect("authored query");
    assert_eq!(
        service::audit(&local(), &oracle, &tool)
            .expect_err("negative-only authored evidence cannot certify")
            .to_string(),
        "a rule requires positive and negative cases"
    );
}
fn missing_defaults_remain_missing_and_empty_inventory_is_refused() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("rules")).unwrap();
    std::fs::write(
        root.path().join("benchmark.toml"),
        "tool='fixture'\nversion='1'\nsha256=['pin']\ndefaults=['fixture']\n",
    )
    .unwrap();
    let suite = local();
    let mut tool = suite.profile(ArtifactPath(root.path())).unwrap();
    let audit = service::audit(&suite, root.path(), &tool).unwrap();
    assert!(audit.implemented.is_empty());
    assert_eq!(audit.missing, tool.defaults);
    tool.defaults.clear();
    assert_eq!(
        service::audit(&suite, root.path(), &tool)
            .unwrap_err()
            .to_string(),
        "empty or duplicated default rule inventory"
    );
}

fn negative_only_evidence_is_not_authored_coverage() {
    let root = tempfile::tempdir().unwrap();
    negative_only_evidence(root.path());
}

type Scenario = fn();
type Input = Scenario;
type Value = ();
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("negative_only_evidence_is_not_authored_coverage"),
        word: Word::Succeeds,
        input: || negative_only_evidence_is_not_authored_coverage as fn(),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("missing_defaults_remain_missing_and_empty_inventory_is_refused"),
        word: Word::Succeeds,
        input: || missing_defaults_remain_missing_and_empty_inventory_is_refused as fn(),
        expect: Expect::Value(()),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
