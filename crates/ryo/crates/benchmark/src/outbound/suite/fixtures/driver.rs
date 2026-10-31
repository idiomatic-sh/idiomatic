//! Independent artifact construction shared by acceptance proofs.
use crate::domain::suite::types::Tool;
use crate::outbound::suite::{interface::SuitePort, types::ArtifactPath};
/// Produce the acceptance fixture through the real artifact boundary.
pub fn audited(complete: bool) -> crate::domain::suite::types::audit::value::Audit {
    let directory = tempfile::tempdir().expect("audit fixture");
    std::fs::create_dir(directory.path().join("rules")).expect("rule root");
    if complete {
        let rule = directory.path().join("rules/fixture");
        std::fs::create_dir(&rule).expect("rule directory");
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
        tool: "fixture".into(),
        version: "1".into(),
        sha256: vec![String::new().into()],
        deferred: Default::default(),
        pending: Default::default(),
        defaults: vec!["fixture".try_into().expect("rule identifier")],
    };
    crate::outbound::suite::factory::local()
        .audit(ArtifactPath(directory.path()), &tool)
        .expect("audited fixture")
}
