//! Reports distinguish planned writes from completed initialization.
use super::answer;
use crate::cases::{Case, CaseName, Expect, Word, run};
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::init::types::InitReport;
type Input = bool;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("planned"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("written"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(true),
    },
];
fn subject(dry_run: Input) -> Result<Value, Failure> {
    let answer = answer(
        InitReport {
            written: vec![RepoPath("tastemaker.toml".into())],
        },
        dry_run,
    );
    let text = answer.to_string();
    // `--format json` prints the bytes serde_json writes of the answer, on one line, and SARIF's spacing is serde_json's
    // indented form.
    assert_eq!(
        encoded(&answer, crate::output::Spacing::Compact),
        serde_json::to_vec(&answer).unwrap_or_default()
    );
    assert_eq!(
        encoded(&answer, crate::output::Spacing::Indented),
        serde_json::to_vec_pretty(&answer).unwrap_or_default()
    );
    assert_eq!(answer.dry_run, dry_run);
    assert!(text.starts_with(if dry_run { "would write" } else { "wrote" }));
    assert_eq!(text.contains("next:"), !dry_run);
    // Each step names a command the command line has, and the top-level help points here.
    let tree = crate::resources::command();
    for step in super::NEXT {
        let command = step.split_whitespace().nth(1).unwrap_or_default();
        assert!(tree.find_subcommand(command).is_some(), "{step}");
    }
    assert!(tree.find_subcommand("init").is_some());
    assert!(
        tree.clone()
            .render_long_help()
            .to_string()
            .contains("Get started: tm init")
    );
    let json = serde_json::to_value(&answer).map_err(|_error| ())?;
    Ok(json["dry_run"].as_bool() == Some(dry_run))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// What `answer` writes as JSON, spaced as `spacing` says.
fn encoded(answer: &impl crate::output::Encoded, spacing: crate::output::Spacing) -> Vec<u8> {
    let mut written = Vec::new();
    answer.json(&mut written, spacing).unwrap_or_default();
    written
}
