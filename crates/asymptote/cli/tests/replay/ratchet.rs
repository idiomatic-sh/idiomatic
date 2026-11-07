//! The ratchet of the built binary over a tool that lists no rules (as ruff's SARIF does): an accepted finding that is
//! fixed is resolved and drained, and brought back it is new and fails; a check that did not run a record's tool
//! resolves nothing of it.
#![cfg(unix)]
#![expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be set up is a broken test, and the message names which"
)]

use std::path::Path;
use std::process::{Command, Output};

/// A SARIF log that reports one `F401` in `a.py`.
const FOUND: &str = r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"results":[{"ruleId":"F401","level":"error","message":{"text":"unused"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.py"},"region":{"startLine":1}}}]}]}]}"#;

/// A SARIF log that reports nothing.
const CLEAN: &str =
    r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"lint"}},"results":[]}]}"#;

/// Two tools, each in its own group, each printing the log in its own file.
const MANIFEST: &str = "[[tools]]\nid = \"one\"\ngroup = \"a\"\nrun = [\"cat\", \"one.sarif\"]\noutput = \"sarif\"\n\n[[tools]]\nid = \"two\"\ngroup = \"b\"\nrun = [\"cat\", \"two.sarif\"]\noutput = \"sarif\"\n";

/// Runs git in `path`.
fn git(path: &Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {arguments:?}");
}

/// Runs asymptote in `path`.
fn asymptote(path: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asymptote"))
        .args(arguments)
        .current_dir(path)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .expect("asymptote runs")
}

/// What `output` printed to stdout and stderr.
fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A repository whose two tools both report the finding, accepted.
fn accepted() -> tempfile::TempDir {
    let product = tempfile::tempdir().expect("a temporary directory");
    let path = product.path();
    git(path, &["init", "-q"]);
    git(path, &["config", "user.email", "t@example.com"]);
    git(path, &["config", "user.name", "t"]);
    std::fs::write(path.join("asymptote.toml"), MANIFEST).expect("the manifest");
    std::fs::write(path.join("a.py"), "import os\n").expect("the source");
    for log in ["one.sarif", "two.sarif"] {
        std::fs::write(path.join(log), FOUND).expect("the log");
    }
    git(path, &["add", "-A"]);
    git(path, &["commit", "-q", "-m", "init"]);
    let first = asymptote(path, &["check"]);
    assert!(
        !first.status.success(),
        "the findings are new: {}",
        said(&first)
    );
    let up = asymptote(path, &["up", "--reason", "backlog"]);
    assert!(up.status.success(), "up accepts: {}", said(&up));
    product
}

#[test]
fn a_fixed_finding_of_a_tool_that_lists_no_rules_is_resolved_and_drained() {
    let product = accepted();
    let path = product.path();
    std::fs::write(path.join("one.sarif"), CLEAN).expect("the log");
    std::fs::write(path.join("two.sarif"), CLEAN).expect("the log");
    let fixed = asymptote(path, &["check"]);
    let text = said(&fixed);
    assert!(text.contains("2 resolved"), "resolved: {text}");
    assert!(!text.contains("unjudged"), "judged: {text}");
    let down = asymptote(path, &["down"]);
    assert!(down.status.success(), "down: {}", said(&down));
    assert!(said(&down).contains("2 drained"), "{}", said(&down));
    // Brought back, the finding is new: the lock no longer holds it.
    std::fs::write(path.join("one.sarif"), FOUND).expect("the log");
    let back = asymptote(path, &["check"]);
    assert!(
        !back.status.success(),
        "a returned finding fails: {}",
        said(&back)
    );
    assert!(said(&back).contains("1 new"), "{}", said(&back));
}

#[test]
fn a_check_that_did_not_run_a_tool_resolves_nothing_of_its_records() {
    let product = accepted();
    let path = product.path();
    std::fs::write(path.join("one.sarif"), CLEAN).expect("the log");
    std::fs::write(path.join("two.sarif"), CLEAN).expect("the log");
    let narrowed = asymptote(path, &["check", "--groups", "a"]);
    let text = said(&narrowed);
    assert!(text.contains("1 resolved"), "only group a resolves: {text}");
    let down = asymptote(path, &["down"]);
    assert!(said(&down).contains("1 drained"), "{}", said(&down));
    // The tool of group b never ran: its record stands in the lock, and the finding is still accepted.
    std::fs::write(path.join("two.sarif"), FOUND).expect("the log");
    let later = asymptote(path, &["check", "--groups", "b"]);
    assert!(
        later.status.success(),
        "group b's record stands: {}",
        said(&later)
    );
}
