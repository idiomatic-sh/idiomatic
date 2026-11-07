//! The targets' findings: the performance job (`benches/bench.rs`) runs each target that states
//! its expected findings in a fresh copy of its pinned revision, through this file, and holds
//! what each run prints to the target's `findings.toml`. Nothing here is timed, and no smoke
//! test calls it.

use super::{output, run};
#[path = "proof/contract_test.rs"]
mod contract;
#[path = "proof/preparation_test.rs"]
pub(crate) mod preparation;
#[path = "proof/summary_test.rs"]
mod summary;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The name of the product's configuration file: a checkout places the target's
/// `configuration.toml` in the project under it.
const CONFIGURATION: &str = "";

/// The arguments that make `ryo` check the working directory and print its
/// findings on standard output.
pub(crate) const CHECK: &str = "query run --format json --text 'let marked = select * from \"comment\" where glob_any(text, \"*distribution-proof*\"); emit marked subject file key(span);' ";

/// Installs a registry's package under a scratch directory and returns the installed
/// command.
type Install = fn(&Path) -> std::io::Result<PathBuf>;

/// The languages a registry's build links.
type Languages = fn() -> std::io::Result<Vec<String>>;

/// The registries this product ships to, one module each in the test root, each with its
/// package's install and the languages its build links.
pub(crate) const REGISTRIES: &[(&str, Install, Languages)] =
    &[("cargo", super::cargo::command, super::cargo::languages)];

/// `registry`'s targets, in order; a registry with none is refused.
pub(crate) fn targets(registry: &str) -> std::io::Result<Vec<PathBuf>> {
    let mut targets = std::fs::read_dir(super::registry_directory(registry).join("targets"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    targets.sort();
    if targets.is_empty() {
        return Err(std::io::Error::other(format!("{registry} has no target")));
    }
    Ok(targets)
}

/// What a target that can run states beside its pin.
const RUNNABLE: [&str; 3] = ["findings.toml", "edit.patch", "edit-findings.toml"];

/// Whether the target states its expected findings, `findings.toml`; not while it states its pin
/// alone, as the spec places it until every target can have findings. A
/// target with any evidence must have all three files, an edit of one file,
/// and an edit that changes its findings.
pub(crate) fn expected(target: &Path) -> std::io::Result<bool> {
    let missing = RUNNABLE
        .iter()
        .filter(|file| !target.join(file).is_file())
        .collect::<Vec<_>>();
    if missing.len() == RUNNABLE.len() && !target.join("configuration.toml").exists() {
        return Ok(false);
    }
    if let Some(file) = missing.first() {
        return Err(std::io::Error::other(format!("it has no `{file}`")));
    }
    if !CONFIGURATION.is_empty() && !target.join("configuration.toml").is_file() {
        return Err(std::io::Error::other("it has no `configuration.toml`"));
    }
    let edit = std::fs::read_to_string(target.join("edit.patch"))?;
    let files = edit
        .lines()
        .filter(|line| line.starts_with("diff --git "))
        .count();
    if files != 1 {
        return Err(std::io::Error::other(format!(
            "its edit changes {files} files, not one"
        )));
    }
    if summary::read(&target.join("findings.toml"))?
        .matches(&summary::read(&target.join("edit-findings.toml"))?)
    {
        return Err(std::io::Error::other(
            "its edit changes no finding: `edit-findings.toml` is `findings.toml`",
        ));
    }
    Ok(true)
}

/// Prepares the existing policy used by Tastemaker and the SARIF ratchet target.
fn prepare_policy(_command: &Path, project: &Path) -> std::io::Result<()> {
    if !project.join(".policy").is_dir() {
        return Ok(());
    }

    let installation = project.join(".tastemaker");
    run(Command::new("cargo")
        .args(["install", "--locked", "--profile", "gate", "--path"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tastemaker/cli"))
        .arg("--root")
        .arg(&installation))?;
    let tastemaker = installation
        .join("bin")
        .join(format!("tm{}", std::env::consts::EXE_SUFFIX));
    let tastemaker = tastemaker.as_path();

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
    if !CONFIGURATION.is_empty() {
        let _: u64 = std::fs::copy(
            target.join("configuration.toml"),
            project.join(CONFIGURATION),
        )?;
    }
    prepare_policy(command, &project)?;
    Ok((tree, project))
}

/// One run of `command` over a fresh checkout of `target`, held to `findings`: what it
/// prints, and the exit code it ends with, which must be the summary's `exit_code`.
fn check(command: &Path, target: &Path, snapshot: &Path, findings: &Path) -> std::io::Result<u64> {
    let scratch = tempfile::tempdir()?;
    let (tree, project) = checkout(command, target, snapshot, scratch.path())?;
    if findings
        .file_name()
        .is_some_and(|name| name == "edit-findings.toml")
    {
        run(Command::new("git")
            .arg("apply")
            .arg(target.join("edit.patch"))
            .current_dir(&tree))?;
    }
    let output = Command::new(command)
        .args(shlex::split(CHECK).ok_or_else(|| std::io::Error::other("invalid check arguments"))?)
        .current_dir(&project)
        .output()?;
    hold(&output.stdout, output.status.code(), findings).map_err(|error| {
        std::io::Error::other(format!(
            "{error}\n{}",
            String::from_utf8_lossy(&output.stderr)
        ))
    })
}

/// Installs once when needed and reports each target's result or missing evidence.
/// Only errors fail targets; a slow run remains informational.
pub(crate) fn each(
    registry: &str,
    targets: &[PathBuf],
    install: impl FnOnce() -> std::io::Result<PathBuf>,
    run: impl Fn(&Path, &Path) -> std::io::Result<String>,
    output: &mut impl Write,
) -> std::io::Result<Vec<String>> {
    let mut failed = Vec::new();
    let mut runnable = Vec::new();
    for target in targets {
        let name = target
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        match expected(target) {
            Ok(true) => runnable.push((name, target)),
            Ok(false) => writeln!(
                output,
                "{registry} {name}: no expected findings yet: it states its pin alone"
            )?,
            Err(error) => failed.push(format!("{registry} {name}: cannot run: {error}")),
        }
    }
    if !runnable.is_empty() {
        let command = install()?;
        for (name, target) in runnable {
            match run(&command, target) {
                Ok(line) => writeln!(output, "{registry} {name}: {line}")?,
                Err(error) => failed.push(format!("{registry} {name}: cannot run: {error}")),
            }
        }
    }
    for line in &failed {
        writeln!(output, "{line}")?;
    }
    Ok(failed)
}

/// Every registry the test root declares has a target to run: a directory under its
/// `targets`. Nothing is run here; the bench runs each one.
#[test]
fn every_registry_has_a_target() -> std::io::Result<()> {
    for (registry, ..) in REGISTRIES {
        assert!(!targets(registry)?.is_empty(), "{registry} has no target");
    }
    Ok(())
}

/// Each directory under `registries` is a registry the product ships to, so no registry's
/// package sits there with its smoke test uncompiled.
#[test]
fn every_registry_is_declared() {
    let mut found = std::fs::read_dir(super::distribution_directory().join("registries"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    found.sort();
    let mut declared = REGISTRIES
        .iter()
        .map(|(registry, ..)| *registry)
        .collect::<Vec<_>>();
    declared.sort_unstable();
    assert_eq!(found, declared);
}

/// A target that states its pin alone is reported and nothing is installed for
/// it. A run held to a copy of its expected findings passes however slow it is, and so does
/// a run that answers it is slow; once the copy is edited, the same run fails the
/// target and names what changed.
#[test]
fn findings_mismatches_are_rejected_independently_of_speed() -> std::io::Result<()> {
    let scratch = tempfile::tempdir()?;
    let printed = r#"{"findings":[{"rule":"a","path":"x"},{"rule":"b","path":"y"}]}"#;
    let snapshot = scratch.path().join("snapshot");
    run(Command::new("git").args(["init", "--quiet"]).arg(&snapshot))?;
    std::fs::write(snapshot.join("source.txt"), "x\n")?;
    run(Command::new("git")
        .args(["add", "."])
        .current_dir(&snapshot))?;
    run(Command::new("git")
        .args(["-c", "user.name=proof", "-c", "user.email=proof@invalid"])
        .args(["commit", "--quiet", "--message", "the target"])
        .current_dir(&snapshot))?;
    let unrunnable = scratch.path().join("unrunnable");
    std::fs::create_dir(&unrunnable)?;
    let refused = || Err(std::io::Error::other("nothing is installed for no target"));
    let unrun = |_: &Path, _: &Path| Err(std::io::Error::other("no target is run"));
    let mut said = Vec::new();
    assert!(each("proof", &[unrunnable], refused, unrun, &mut said)?.is_empty());
    assert!(String::from_utf8_lossy(&said).contains("proof unrunnable: no expected findings yet"));
    let target = scratch.path().join("target");
    std::fs::create_dir(&target)?;
    let revision = output(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&snapshot),
    )?;
    let pin = toml::Table::from_iter([
        ("directory".to_owned(), toml::Value::String(".".to_owned())),
        (
            "revision".to_owned(),
            toml::Value::String(revision.trim().to_owned()),
        ),
        (
            "repository".to_owned(),
            toml::Value::String(snapshot.to_string_lossy().into_owned()),
        ),
    ]);
    std::fs::write(target.join("pin.toml"), pin.to_string())?;
    let cached = preparation::snapshot(&target)?;
    assert_eq!(std::fs::read_to_string(cached.join("source.txt"))?, "x\n");
    std::fs::write(target.join("configuration.toml"), "")?;
    let findings = target.join("findings.toml");
    std::fs::write(&findings, summary::of(printed.as_bytes(), 1)?.text())?;
    std::fs::write(
        target.join("edit.patch"),
        "diff --git a/source.txt b/source.txt\n",
    )?;
    std::fs::write(
        target.join("edit-findings.toml"),
        summary::of(br#"{"findings":[]}"#, 0)?.text(),
    )?;
    // A command that prints `printed` and ends with `code`: a shell script, and on Windows a batch file, which the system runs
    // through cmd.exe.
    let command = scratch.path().join(if cfg!(windows) {
        "command.cmd"
    } else {
        "command"
    });
    let script = |code: i32| {
        if cfg!(windows) {
            format!("@echo off\r\necho {printed}\r\nexit /b {code}\r\n")
        } else {
            format!("#!/bin/sh\nsleep 0.05\nprintf '%s' '{printed}'\nexit {code}\n")
        }
    };
    std::fs::write(&command, script(1_i32))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700))?;
    }
    let targets = [target.clone()];
    let install = || Ok(command.clone());
    let held = |command: &Path, target: &Path| {
        let total = check(command, target, &snapshot, &target.join("findings.toml"))?;
        Ok(format!("findings hold: {total}"))
    };
    assert!(each("proof", &targets, install, held, &mut said)?.is_empty());
    assert!(String::from_utf8_lossy(&said).contains("proof target: findings hold: 2"));
    let over = |_: &Path, _: &Path| Ok("cold 900 ms".to_owned());
    assert!(each("proof", &targets, install, over, &mut said)?.is_empty());
    // The same findings with another exit code than the summary's `exit_code` fail the target.
    std::fs::write(&command, script(0_i32))?;
    let failed = each("proof", &targets, install, held, &mut said)?;
    assert!(
        failed.len() == 1 && failed[0].contains("ended with exit code 0, not the `exit_code = 1`"),
        "{failed:?}"
    );
    std::fs::write(&command, script(1_i32))?;
    // The edit: the copy's digest, the identity of the whole findings document, changed.
    let copy = std::fs::read_to_string(&findings)?;
    let digest = copy.find("digest = \"").map(|at| at + "digest = \"".len());
    let digest = digest.ok_or_else(|| std::io::Error::other("the summary has no digest"))?;
    let changed = if copy[digest..].starts_with('0') {
        "1"
    } else {
        "0"
    };
    std::fs::write(
        &findings,
        [&copy[..digest], changed, &copy[digest + 1..]].concat(),
    )?;
    let failed = each("proof", &targets, install, held, &mut said)?;
    assert!(
        failed.len() == 1 && failed[0].contains("its findings are not those of"),
        "{failed:?}"
    );
    std::fs::remove_file(target.join("configuration.toml"))?;
    assert!(
        expected(&target).is_err() != CONFIGURATION.is_empty(),
        "only products with configuration require a target configuration"
    );
    Ok(())
}
