//! Git operations used only to construct independent workspace fixtures.
use std::{path::Path, process::Command};

/// Execute a fixture setup command and require successful UTF-8 output.
#[expect(
    clippy::expect_used,
    reason = "fixture artifact setup must succeed before testing the port"
)]
pub(crate) fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("fixture git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("utf8")
        .trim()
        .into()
}
