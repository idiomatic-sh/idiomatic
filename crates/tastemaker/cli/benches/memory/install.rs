//! The command the bench measures: the command line crate installed under a scratch root by Cargo's
//! install, in the release profile with its default features, as a user installs it from crates.io;
//! and the runs of a command to completion that the install, the checkout and the measures share.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Installs the command line crate under `root` and returns the installed command.
pub(super) fn cargo_install(root: &Path) -> std::io::Result<PathBuf> {
    run(Command::new("cargo")
        .args(["install", "--locked", "--profile", "release", "--path"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--root")
        .arg(root))?;
    Ok(root
        .join("bin")
        .join(format!("tm{}", std::env::consts::EXE_SUFFIX)))
}

/// Runs `command` to completion for its effect, or answers an error carrying both of its
/// outputs when it fails.
pub(crate) fn run(command: &mut Command) -> std::io::Result<()> {
    output(command).map(drop)
}

/// Runs `command` to completion and answers its standard output, or an error carrying both
/// of its outputs when it fails.
pub(crate) fn output(command: &mut Command) -> std::io::Result<String> {
    let ran = command.output()?;
    if !ran.status.success() {
        return Err(std::io::Error::other(format!(
            "{command:?} failed with {}:\n{}{}",
            ran.status,
            String::from_utf8_lossy(&ran.stdout),
            String::from_utf8_lossy(&ran.stderr)
        )));
    }
    String::from_utf8(ran.stdout).map_err(std::io::Error::other)
}
