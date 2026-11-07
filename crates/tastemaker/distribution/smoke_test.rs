//! tastemaker's distribution: one build per registry it ships to, each in its registry's
//! directory under `registries` beside the smoke test that builds that registry's artifact
//! from the command line crate, installs it in a scratch directory and runs
//! `tm --version`. It runs no target: each target, its findings and its performance are the performance
//! job's (`benches/bench.rs`). Nothing is published.

#[path = "registries/cargo/smoke_test.rs"]
mod cargo;
#[path = "registries/npm/smoke_test.rs"]
mod npm;
#[path = "registries/pypi/smoke_test.rs"]
mod pypi;
pub(crate) mod target_test;

use std::path::{Path, PathBuf};
use std::process::Command;

/// What `tm --version` prints: the crate's own version.
const VERSION: &str = concat!("tm ", env!("CARGO_PKG_VERSION"), "\n");

/// Holds the installed product's version, which a product that links providers follows
/// with them in parentheses.
fn has_version(printed: &str) -> bool {
    let version = VERSION.trim_end();
    printed.strip_suffix('\n').is_some_and(|line| {
        line == version
            || line
                .strip_prefix(version)
                .is_some_and(|rest| rest.starts_with(" (") && rest.ends_with(')'))
    })
}
/// Reads the languages the installed command links: the providers its `--version` lists in
/// parentheses, else those of its provider catalog.
fn linked(command: &Path) -> std::io::Result<Vec<String>> {
    let printed = output(Command::new(command).arg("--version"))?;
    if let Some((_, languages)) = printed
        .strip_suffix(")\n")
        .and_then(|text| text.split_once(" ("))
    {
        if languages.is_empty() {
            return Ok(Vec::new());
        }
        return languages
            .split(", ")
            .map(|language| {
                language
                    .split_once(' ')
                    .map(|(name, _)| name.to_owned())
                    .ok_or_else(|| std::io::Error::other("the version has an invalid provider"))
            })
            .collect();
    }
    let described =
        output(Command::new(command).args(["relation", "describe", "comment", "--json"]))?;
    let catalog: serde_json::Value =
        serde_json::from_str(&described).map_err(std::io::Error::other)?;
    catalog
        .get("languages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("the catalog has no languages"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .and_then(|text| text.split_once(": "))
                .map(|(name, _)| name.to_owned())
                .ok_or_else(|| std::io::Error::other("the catalog has an invalid language"))
        })
        .collect()
}

/// The product's `distribution` directory, which holds this file, the targets' runner and
/// one directory per registry, beside the command line crate whose `distribution` test
/// target this file is.
fn distribution_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../distribution")
}

/// The directory of `registry`'s build, package files, targets and smoke test.
fn registry_directory(registry: &str) -> PathBuf {
    distribution_directory().join("registries").join(registry)
}

/// Runs `command` to completion for its effect, or answers an error carrying both of its
/// outputs when it fails.
pub(crate) fn run(command: &mut Command) -> std::io::Result<()> {
    let ran = command.output()?;
    if ran.status.success() {
        Ok(())
    } else {
        Err(failure(command, &ran))
    }
}

/// Runs `command` to completion and answers its standard output, or an error carrying both
/// of its outputs when it fails.
pub(crate) fn output(command: &mut Command) -> std::io::Result<String> {
    let ran = command.output()?;
    if !ran.status.success() {
        return Err(failure(command, &ran));
    }
    String::from_utf8(ran.stdout).map_err(std::io::Error::other)
}

/// The error of `command`, which ended with `ran`: its status and both of its outputs.
fn failure(command: &Command, ran: &std::process::Output) -> std::io::Error {
    std::io::Error::other(format!(
        "{command:?} failed with {}:\n{}{}",
        ran.status,
        String::from_utf8_lossy(&ran.stdout),
        String::from_utf8_lossy(&ran.stderr)
    ))
}

/// Installs the command line crate under `root` with `cargo install`, in the workspace's
/// `profile`, linking `languages` alone when it names any and the crate's defaults
/// otherwise, and returns the installed command.
fn cargo_install(root: &Path, profile: &str, languages: &[String]) -> std::io::Result<PathBuf> {
    let features = if languages.is_empty() {
        Vec::new()
    } else {
        vec![
            "--no-default-features".to_owned(),
            "--features".to_owned(),
            languages.join(","),
        ]
    };
    run(Command::new("cargo")
        .args(["install", "--locked", "--profile", profile, "--path"])
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--root")
        .arg(root)
        .args(features))?;
    let binary = format!("tm{}", std::env::consts::EXE_SUFFIX);
    Ok(root.join("bin").join(binary))
}

/// The host's target triple, as rustc names it.
fn host() -> std::io::Result<String> {
    output(Command::new("rustc").arg("-vV"))?
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other("`rustc -vV` names no host"))
}

/// The strings of the array at `keys`, a path of table keys, in the TOML file `file`: none
/// when the file has no such array.
fn strings(file: &Path, keys: &[&str]) -> std::io::Result<Vec<String>> {
    let document = toml::Value::Table(
        std::fs::read_to_string(file)?
            .parse()
            .map_err(std::io::Error::other)?,
    );
    Ok(keys
        .iter()
        .try_fold(&document, |value, key| value.get(*key))
        .and_then(toml::Value::as_array)
        .map(|elements| {
            elements
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}
