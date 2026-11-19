//! The target's checkout and its findings guard: a fresh copy of the target's pinned revision, with
//! its dependencies bound, its configuration placed and its policy prepared, and the hold of what
//! a run prints, and the exit code it ends with, to the target's `findings.toml`.

pub(crate) mod preparation;
pub(crate) mod summary;

use super::install::run;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The name of the product's configuration file: a checkout places the target's
/// `configuration.toml` in the project under it.
const CONFIGURATION: &str = "tastemaker.toml";

/// The arguments that make `tm` check the working directory and print its
/// findings on standard output.
pub(crate) const CHECK: &str = "check --format json";

/// Prepares the existing policy used by Tastemaker and the SARIF ratchet target.
fn prepare_policy(command: &Path, project: &Path) -> std::io::Result<()> {
    if !project.join(".policy").is_dir() {
        return Ok(());
    }

    let tastemaker = command;

    run(Command::new(tastemaker)
        .args([
            "validate",
            "idiomatic.arch/rule/file-length@0.3.0",
            "--json",
        ])
        .current_dir(project))?;
    // The fix writes the lock. It ends with 1 when the target has findings, which the run holds
    // to its summary later; only an error (2) fails the preparation.
    let fixed = Command::new(tastemaker)
        .args(["check", "--fix", "--format", "json"])
        .current_dir(project)
        .output()?;
    if !matches!(fixed.status.code(), Some(0_i32 | 1_i32)) {
        return Err(std::io::Error::other(format!(
            "`tm check --fix` ended with {}: {}",
            fixed.status,
            String::from_utf8_lossy(&fixed.stderr)
        )));
    }
    Ok(())
}

/// Holds what the command printed, and the exit code it `ended` with, to the summary
/// `expected` states and answers their total; on a mismatch names the per-rule changes.
pub(crate) fn hold(printed: &[u8], ended: Option<i32>, expected: &Path) -> std::io::Result<u64> {
    let wanted = summary::read(expected)?;
    if ended != Some(wanted.exit_code()) {
        return Err(std::io::Error::other(format!(
            "the check ended with exit code {}, not the `exit_code = {}` of {}",
            ended.map_or_else(|| "none".to_owned(), |code| code.to_string()),
            wanted.exit_code(),
            expected.display()
        )));
    }
    let actual = summary::of(printed, wanted.exit_code())?;
    if actual.matches(&wanted) {
        return Ok(wanted.total());
    }
    let changes = actual.changes(&wanted).join("\n");
    Err(std::io::Error::other(format!(
        "its findings are not those of {}:\n{changes}",
        expected.display()
    )))
}

/// A fresh checkout of `target` under `scratch`, from its cached `snapshot`, with its dependencies bound, its
/// configuration placed and its policy prepared: the checkout's root and the project the command runs in.
pub(crate) fn checkout(
    command: &Path,
    target: &Path,
    snapshot: &Path,
    scratch: &Path,
) -> std::io::Result<(PathBuf, PathBuf)> {
    let (tree, project) = preparation::checkout(target, snapshot, scratch)?;
    let _: u64 = std::fs::copy(
        target.join("configuration.toml"),
        project.join(CONFIGURATION),
    )?;
    prepare_policy(command, &project)?;
    Ok((tree, project))
}
