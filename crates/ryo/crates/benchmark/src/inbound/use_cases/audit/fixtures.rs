//! Independent artifact construction shared by acceptance proofs.
use crate::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
/// Produce the acceptance fixture through the real artifact boundary.
///
/// # Panics
///
/// Panics when fixed fixture artifacts cannot be created or validated.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "fixture artifact setup must succeed before testing the port"
)]
pub fn audited(complete: bool) -> crate::domain::suite::types::audit::value::Audit {
    let directory = tempfile::tempdir().expect("audit fixture");
    std::fs::create_dir_all(directory.path().join("rules")).expect("rule root");
    if complete {
        let rule = directory.path().join("rules/fixture");
        std::fs::create_dir_all(&rule).expect("rule directory");
        std::fs::write(
            rule.join("query.ryoql"),
            "let nodes = select * from \"syntax-tree\"; emit nodes subject file key(id);",
        )
        .expect("query");
        std::fs::write(
            rule.join("cases.toml"),
            r#"[[cases]]
name = "positive"
files = { "input.txt" = "x" }
expected = [{file="input.txt",start=0,end=1}]
[[cases]]
name = "negative"
files = { "input.txt" = "" }
expected = []
"#,
        )
        .expect("cases");
    }
    let tool = Tool {
        rule_observation_selection:
            ryo_benchmark_contracts::domain::context::types::ObservationSelectionPolicy::Declared,
        supporting_rules: Vec::new(),
        rule_observations: Default::default(),
        rule_context: ryo_benchmark_contracts::domain::workloads::types::RuleContextProfile::Undeclared,
        file_observation: ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared,
        project_context_mode:
            ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified,
        source_language: ryo_benchmark_contracts::domain::requests::types::ProviderSelection::Unspecified,
        project_search_roots: ryo_benchmark_contracts::domain::requests::types::SearchRoots::Undeclared,
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    };
    crate::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::suite::factory::local(),
    ))
    .audit(directory.path(), &tool)
    .expect("audited fixture")
}
