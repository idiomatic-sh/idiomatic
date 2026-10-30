//! What every command that reads or writes a project's files is run with, over each sample under `crates/tastemaker/examples/<language>/api`:
//! the commands hold in any language a project is written in, not only in the Rust of this repository. Each command's cases
//! are in its own module of the suite. A sample is found by listing
//! `crates/tastemaker/examples/`, so a language is added by adding its sample and nothing here. Each case starts from a copy of the sample's
//! own files (its sources, tests and manifests, without its Tastemaker files) in a repository of its own, the way a project
//! that adopts Tastemaker starts, and asks only what holds in every language: the source extension is the one most of the
//! project's files have, its Tastemaker setup left out, as `tm create` counts them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::fixes::{CLEAN, FOUND};

/// The files of a sample that are its Tastemaker setup, left out of the project a case starts from.
const SETUP: [&str; 4] = [
    "tastemaker.toml",
    "tastemaker.lock",
    "tastemaker",
    "fixes.toml",
];

/// The rule each case creates, and the template.
pub(crate) const RULE: &str = "empty-comment";
pub(crate) const TEMPLATE: &str = "notes";

/// A sample: its name (`<language>/api`), its directory, and the extension most of its source files have.
pub(crate) struct Sample {
    pub name: String,
    pub directory: PathBuf,
    pub extension: String,
}

/// The repository root: three directories above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// The extension most of a sample's project files have: every file but its Tastemaker setup and what a run or a package
/// manager writes, as `tm create` counts a project's files; a file with no extension counts for none. A directory that
/// cannot be listed is the error, never one with no files.
fn most_common_extension(directory: &Path) -> std::io::Result<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut pending = vec![(directory.to_path_buf(), true)];
    while let Some((next, top)) = pending.pop() {
        for entry in std::fs::read_dir(&next)? {
            let entry = entry?;
            let name = entry.file_name();
            if crate::fixes::UNCOPIED
                .iter()
                .any(|left_out| name == *left_out)
                || (top && SETUP.iter().any(|left_out| name == *left_out))
            {
                continue;
            }
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                // A link is no file of the sample's own: its target is counted where it is.
            } else if kind.is_dir() {
                pending.push((path, false));
            } else if let Some(extension) = path.extension() {
                *counts
                    .entry(extension.to_string_lossy().into_owned())
                    .or_default() += 1;
            } else {
                // A file with no extension counts for none.
            }
        }
    }
    Ok(counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then(right.0.cmp(&left.0)))
        .map(|(extension, _)| extension)
        .unwrap_or_default())
}

/// Every sample under `crates/tastemaker/examples/`: each `crates/tastemaker/examples/<language>/api` that holds a `tastemaker.toml`, in order of name.
#[expect(
    clippy::expect_used,
    reason = "test support: a sample that cannot be listed is a broken test, and the message names which"
)]
pub(crate) fn samples() -> Vec<Sample> {
    let examples = root().join("crates/tastemaker/examples");
    let mut found = Vec::new();
    for language in std::fs::read_dir(&examples).expect("the examples directory") {
        let directory = language.expect("an entry of examples/").path().join("api");
        if !directory.join("tastemaker.toml").is_file() {
            continue;
        }
        let name = directory
            .strip_prefix(&examples)
            .expect("a sample under examples/")
            .to_string_lossy()
            .into_owned();
        let extension = most_common_extension(&directory).expect("the sample's files");
        found.push(Sample {
            name,
            directory,
            extension,
        });
    }
    found.sort_by(|left, right| left.name.cmp(&right.name));
    assert!(!found.is_empty(), "examples/ holds at least one sample");
    found
}

/// Copies a tree, leaving out what a run writes and, at the top, the sample's Tastemaker setup.
fn copy(from: &Path, to: &Path, top: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if crate::fixes::UNCOPIED
            .iter()
            .any(|left_out| name == *left_out)
            || (top && SETUP.iter().any(|left_out| name == *left_out))
        {
            continue;
        }
        let (source, target) = (entry.path(), to.join(&name));
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            crate::fixes::link(&source, &target)?;
        } else if kind.is_dir() {
            copy(&source, &target, false)?;
        } else {
            let _: u64 = std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

/// Runs `tm` with `arguments` in `directory`.
#[expect(
    clippy::expect_used,
    reason = "test support: a run that cannot start is a broken test, and the message names which"
)]
fn tm(arguments: &[&str], directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("tm starts")
}

/// The exit code `tm` ends with in `directory`, none when a signal ended it, and what it printed on stdout and on stderr.
pub(crate) fn ended(arguments: &[&str], directory: &Path) -> (Option<i32>, String, String) {
    let output = tm(arguments, directory);
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Runs `tm`, answering why the run failed when it did.
pub(crate) fn ran(arguments: &[&str], directory: &Path) -> Result<(), String> {
    printed(arguments, directory).map(drop)
}

/// Runs `tm` and answers what it printed, or why the run failed.
pub(crate) fn printed(arguments: &[&str], directory: &Path) -> Result<String, String> {
    let output = tm(arguments, directory);
    let printed = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        Ok(printed)
    } else {
        Err(format!(
            "tm {} failed:\n{printed}{}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

/// The sample's own files, without its Tastemaker setup, in a fresh repository, after `tm init`.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be written is a broken test, and the message names which"
)]
pub(crate) fn initialized(sample: &Sample) -> Result<tempfile::TempDir, String> {
    let scratch = tempfile::tempdir().expect("the scene directory");
    copy(&sample.directory, scratch.path(), true).expect("the sample copied");
    let git = Command::new("git")
        .args(["init", "-q"])
        .arg(scratch.path())
        .status()
        .expect("git starts");
    assert!(git.success(), "git init succeeds");
    ran(&["init"], scratch.path())?;
    Ok(scratch)
}

/// The findings `tm check` reports in `project`: rule, path and line; or why it could not say: a report that does not read
/// or holds no `findings`, a finding without its rule or path, or an exit code that is not the one its findings call for.
pub(crate) fn reported(project: &Path) -> Result<Vec<(String, String, Option<i64>)>, String> {
    let checked = tm(&["check", "--format", "json"], project);
    let answer: serde_json::Value = serde_json::from_slice(&checked.stdout).map_err(|error| {
        format!(
            "tm check printed no report ({error}):\n{}",
            String::from_utf8_lossy(&checked.stderr)
        )
    })?;
    let entries = answer
        .get("findings")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| format!("tm check printed a report with no findings list:\n{answer}"))?;
    let mut found = Vec::with_capacity(entries.len());
    for entry in entries {
        let text = |key: &str| {
            entry
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| format!("a finding has no `{key}`: {entry}"))
        };
        found.push((
            text("rule")?,
            text("path")?,
            entry.get("line").and_then(serde_json::Value::as_i64),
        ));
    }
    let expected = if found.is_empty() { CLEAN } else { FOUND };
    if checked.status.code() != Some(expected) {
        return Err(format!(
            "tm check exited {:?} with {} finding(s):\n{}",
            checked.status.code(),
            found.len(),
            String::from_utf8_lossy(&checked.stderr)
        ));
    }
    Ok(found)
}

/// The rule `tm create` wrote in `project`, parsed.
pub(crate) fn created_rule(project: &Path) -> Result<toml::Table, String> {
    let file = project.join(format!("tastemaker/rules/{RULE}/0.1.0/rule.toml"));
    std::fs::read_to_string(&file)
        .map_err(|error| format!("{} cannot be read: {error}", file.display()))?
        .parse::<toml::Table>()
        .map_err(|error| format!("{} does not parse: {error}", file.display()))
}

/// Runs `case` over every sample, collecting a failure named for the sample.
pub(crate) fn over_samples(case: &str, run: impl Fn(&Sample) -> Result<(), String>) {
    let failures: Vec<String> = samples()
        .iter()
        .filter_map(|sample| {
            run(sample)
                .err()
                .map(|why| format!("{}: {case}: {why}", sample.name))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The text of the first file under `project/src` that ends in `extension`, in order of path.
fn source_text(project: &Path, extension: &str) -> Result<String, String> {
    let mut pending = vec![project.join("src")];
    let mut found: Vec<PathBuf> = Vec::new();
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .map_err(|error| format!("cannot list {}: {error}", next.display()))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("cannot list {}: {error}", next.display()))?;
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|each| each == extension) {
                found.push(path);
            } else {
                // Another language's file is no content for this one.
            }
        }
    }
    found.sort();
    let first = found
        .first()
        .ok_or_else(|| format!("the project holds no `.{extension}` file under src/"))?;
    std::fs::read_to_string(first)
        .map_err(|error| format!("{} cannot be read: {error}", first.display()))
}

/// Gives the template `tm create` wrote in `project` content: a new template renders nothing until its author gives it
/// some, and a file the sample already holds, in its language, is the content here and in its passing scenario.
pub(crate) fn given_content(project: &Path, sample: &Sample) -> Result<(), String> {
    let source = source_text(project, &sample.extension)?;
    let template = project.join(format!(
        "tastemaker/templates/{TEMPLATE}/0.1.0/template.toml"
    ));
    let skeleton = std::fs::read_to_string(&template)
        .map_err(|error| format!("the template cannot be read: {error}"))?;
    if !skeleton.contains("content     = ''") || source.contains("'''") {
        return Err(format!("the template's content cannot be set:\n{skeleton}"));
    }
    let scenario = format!("given.files.\"example.{}\"", sample.extension);
    std::fs::write(
        &template,
        skeleton
            .replace(
                "content     = ''",
                &format!("content     = '''\n{source}'''"),
            )
            .replace(
                &format!("{scenario} = \"\""),
                &format!("{scenario} = '''\n{source}'''"),
            ),
    )
    .map_err(|error| format!("the template cannot be written: {error}"))
}

/// Runs `tm create` for `definition` in `project`, naming the sample's extension for its scenario files.
pub(crate) fn created(definition: &str, sample: &Sample, project: &Path) -> Result<(), String> {
    ran(
        &[
            "create",
            definition,
            "--set",
            &format!("extension={}", sample.extension),
        ],
        project,
    )
}

/// The whole sample, its Tastemaker setup included, in a fresh repository: the project as its author keeps it.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be written is a broken test, and the message names which"
)]
pub(crate) fn copied(sample: &Sample) -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("the scene directory");
    copy(&sample.directory, scratch.path(), false).expect("the sample copied");
    let git = Command::new("git")
        .args(["init", "-q"])
        .arg(scratch.path())
        .status()
        .expect("git starts");
    assert!(git.success(), "git init succeeds");
    scratch
}
