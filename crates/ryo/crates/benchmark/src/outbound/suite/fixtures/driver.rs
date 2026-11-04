//! Independent artifact construction shared by acceptance proofs.
use crate::domain::suite::types::Tool;
use crate::outbound::suite::{interface::SuitePort as _, types::ArtifactPath};
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
        supporting_rules: Vec::new(),
        rule_observations: std::collections::BTreeMap::new(),
        rule_context: None,
        file_observation: None,
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: std::collections::BTreeMap::default(),
        pending: std::collections::BTreeMap::default(),
        query_modules: std::collections::BTreeMap::default(),
        rule_applicability: std::collections::BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
        project_search_roots: None,
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    };
    crate::outbound::suite::factory::local()
        .audit(ArtifactPath(directory.path()), &tool)
        .expect("audited fixture")
}
