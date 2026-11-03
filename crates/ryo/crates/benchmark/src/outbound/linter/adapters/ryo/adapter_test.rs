//! The concrete ryo implementation executes the complete shared contract.
use super::adapter::LinterRyoAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::suite::types::RunRequest;
use crate::outbound::linter::fixtures::contract::check;
use crate::outbound::linter::interface::LinterPort as _;
use ryo::linter::RelativePath;
use std::collections::BTreeMap;
type Input = ();
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("concrete implementation answers the shared contract"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(()),
}];
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shared case runner requires a Result-returning subject for every outcome word"
)]
fn subject(_: Input) -> Result<Value, Failure> {
    check(&LinterRyoAdapter {
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        executable: "fixture".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::default(),
        rule_applicability: BTreeMap::default(),
        parser_configuration: None,
        source_language: None,
    });
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn oversized_request_is_staged_as_json_file_instead_of_argv()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let state = root.path().join("state");
    let request = RunRequest {
        file_metadata: None,
        context_manifest: None,
        root: root.path().to_owned(),
        files: (0_usize..8_000_usize)
            .map(|index| RelativePath(format!("sources/module_{index:05}.py").into_boxed_str()))
            .collect(),
        state: state.clone(),
    };
    let adapter = LinterRyoAdapter {
        project_context_mode: crate::domain::suite::types::ProjectContextMode::default(),
        executable: "candidate".into(),
        rules: "rules".into(),
        query_modules: BTreeMap::new(),
        rule_applicability: BTreeMap::new(),
        parser_configuration: Some("python-target=3.10".into()),
        source_language: Some("explicit-fixture-provider".into()),
    };
    let invocation = adapter.prepare(&request)?;
    let request_path = state.join("request.json");
    let bytes = std::fs::read(&request_path)?;
    let decoded: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert!(
        bytes.len() > 132_199,
        "fixture must exceed the Linux argv limit"
    );
    assert_eq!(invocation.arguments[0], "run-ryo");
    assert_eq!(invocation.arguments[1], "--request-file");
    assert_eq!(invocation.arguments[2], request_path.as_os_str());
    assert_eq!(invocation.arguments[3], "python-target=3.10");
    assert_eq!(decoded["files"].as_array().map(Vec::len), Some(8_000));
    assert_eq!(decoded["parser_configuration"], "python-target=3.10");
    assert_eq!(decoded["source_language"], "explicit-fixture-provider");
    assert!(
        invocation
            .arguments
            .iter()
            .all(|argument| argument.len() < 4096)
    );
    let spawned = std::process::Command::new(std::env::current_exe()?)
        .args(&invocation.arguments)
        .output()?;
    assert!(
        spawned.status.code().is_some(),
        "small file-based argv must spawn"
    );
    Ok(())
}
