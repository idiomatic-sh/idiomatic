//! What a sample under `crates/tastemaker/examples/<language>/api` says in its `fixes.toml`, read strictly, and the scene the samples' replay
//! cases run it in: a copy placed in a repository of its own, `tm` run over it, a row's edits applied and put back.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The file of a sample that says what `tm check` reports on it, how each row is fixed, and how the fixed program is tested.
pub(crate) const FIXES: &str = "fixes.toml";

/// The directories a copy of a sample leaves out: what a run writes, and what a package manager or a build fills.
pub(crate) const UNCOPIED: [&str; 8] = [
    ".tastemaker_cache",
    ".git",
    "target",
    "node_modules",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    ".ruff_cache",
];

/// The exit code of a `tm check` that reported an error finding.
pub(crate) const FOUND: i32 = 1;

/// The exit code of a `tm check` that reported none.
pub(crate) const CLEAN: i32 = 0;

/// One finding: the rule, the path from the project's directory, and the line when it has one.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Finding {
    pub rule: String,
    pub path: String,
    pub line: Option<i64>,
}

/// One edit of a row: `find` replaced once by `replace`, a new file written with `replace`, or a file deleted.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Edit {
    pub path: String,
    pub find: Option<String>,
    pub replace: Option<String>,
    #[serde(default)]
    pub delete: bool,
}

/// One row of the README's table: its name, the finding `tm check` reports for it (its rule, its path and, when it has
/// one, its line), and the edits that fix it.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Row {
    #[serde(rename = "row")]
    pub name: String,
    pub rule: String,
    pub path: String,
    pub line: Option<i64>,
    pub edits: Vec<Edit>,
}

/// What a sample's `fixes.toml` says: every key is required.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Fixes {
    pub commands: Vec<Vec<String>>,
    pub rows: Vec<Row>,
}

/// The part of `tm check --format json` the tests read: each finding's rule, path and line, all required.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct Report {
    pub findings: Vec<ReportedFinding>,
}

/// A finding as the report gives it, its path from the repository's root.
#[derive(Debug, serde::Deserialize)]
pub(crate) struct ReportedFinding {
    pub rule: String,
    pub path: String,
    pub line: Option<i64>,
}

/// A sample: its name (`<language>/api`), its directory and what its `fixes.toml` says.
pub(crate) struct Sample {
    pub name: String,
    pub directory: PathBuf,
    pub fixes: Fixes,
}

/// Where a run sees a sample from.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Placement {
    /// A copy at the sample's own path from the repository root, in a repository whose root is the copy's root.
    Nested,
    /// A copy that is the root of a repository of its own.
    Alone,
}

/// The repository root: three directories above this crate.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Every sample under `crates/tastemaker/examples/`, in order of name, or why one cannot be read: each `crates/tastemaker/examples/<language>/api` is one.
#[expect(
    clippy::expect_used,
    reason = "test support: an examples directory that cannot be listed is a broken test"
)]
pub(crate) fn samples() -> Vec<Result<Sample, String>> {
    let examples = root().join("crates/tastemaker/examples");
    let mut directories: Vec<PathBuf> = std::fs::read_dir(&examples)
        .expect("the examples directory")
        .map(|entry| entry.expect("an entry of examples/").path().join("api"))
        .filter(|directory| directory.is_dir())
        .collect();
    directories.sort();
    assert!(
        !directories.is_empty(),
        "examples/ holds at least one sample"
    );
    directories
        .into_iter()
        .map(|directory| sample(&examples, directory))
        .collect()
}

/// The sample in `directory`, or why its `fixes.toml` is missing, malformed or incomplete.
pub(crate) fn sample(examples: &Path, directory: PathBuf) -> Result<Sample, String> {
    let name = directory.strip_prefix(examples).map_or_else(
        |_| directory.display().to_string(),
        |name| name.display().to_string(),
    );
    let text = std::fs::read_to_string(directory.join(FIXES))
        .map_err(|error| format!("{name}: no readable {FIXES}: {error}"))?;
    let fixes: Fixes =
        toml::from_str(&text).map_err(|error| format!("{name}: {FIXES} is malformed: {error}"))?;
    let mut problems = Vec::new();
    for row in &fixes.rows {
        let repairs = fixes
            .rows
            .iter()
            .filter(|other| other.rule == row.rule && other.path == row.path)
            .count();
        if repairs != 1 {
            problems.push(format!(
                "{} at {} has {repairs} rows, not one",
                row.rule, row.path
            ));
        }
    }
    if fixes.rows.is_empty() || fixes.commands.is_empty() {
        problems.push("documents no finding or no command".to_owned());
    }
    if problems.is_empty() {
        Ok(Sample {
            name,
            directory,
            fixes,
        })
    } else {
        Err(format!("{name}: {FIXES}: {}", problems.join("; ")))
    }
}

/// The documented findings of `sample` without those of `left_out`, sorted.
pub(crate) fn documented(sample: &Sample, left_out: Option<&Row>) -> Vec<Finding> {
    let mut findings: Vec<Finding> = sample
        .fixes
        .rows
        .iter()
        .filter(|row| left_out.is_none_or(|out| out.rule != row.rule || out.path != row.path))
        .map(|row| Finding {
            rule: row.rule.clone(),
            path: row.path.clone(),
            line: row.line,
        })
        .collect();
    findings.sort();
    findings
}

/// Copies a tree, leaving out what a run writes.
pub(crate) fn copy(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if UNCOPIED
            .iter()
            .any(|left_out| entry.file_name() == *left_out)
        {
            continue;
        }
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            link(&source, &target)?;
        } else if kind.is_dir() {
            copy(&source, &target)?;
        } else {
            let _: u64 = std::fs::copy(&source, &target)?;
        }
    }
    Ok(())
}

/// A symbolic link of a sample, made again as the link it is, pointing where it pointed: followed, a link to a directory is
/// no file to copy, and one that points outside the sample would copy what is not the sample's.
pub(crate) fn link(source: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(std::fs::read_link(source)?, target)
    }
    #[cfg(not(unix))]
    {
        let _ = (source, target);
        Ok(())
    }
}

/// A copy of `sample` placed as `placement` says, in a fresh repository: the scratch directory that holds it and the project.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be written is a broken test, and the message names which"
)]
pub(crate) fn scene(sample: &Sample, placement: Placement) -> (tempfile::TempDir, PathBuf) {
    let scratch = tempfile::tempdir().expect("the scene directory");
    let project = match placement {
        Placement::Nested => scratch.path().join(
            sample
                .directory
                .strip_prefix(root())
                .expect("a sample inside the repository"),
        ),
        Placement::Alone => scratch.path().to_path_buf(),
    };
    copy(&sample.directory, &project).expect("the sample copied");
    let initialized = Command::new("git")
        .args(["init", "-q"])
        .arg(scratch.path())
        .status()
        .expect("git starts");
    assert!(initialized.success(), "git init succeeds");
    (scratch, project)
}

/// Runs `tm` with `arguments` in `directory`.
#[expect(
    clippy::expect_used,
    reason = "test support: a run that cannot start is a broken test, and the message names which"
)]
pub(crate) fn tm(arguments: &[&str], directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tm"))
        .args(arguments)
        .current_dir(directory)
        .output()
        .expect("tm starts")
}

/// What `tm check` reports in `project` after `tm validate`, each path from the project's directory, sorted; or why it could
/// not say: a failed validation, an exit code that is not the one its findings call for, or a report that does not read.
pub(crate) fn reported(project: &Path) -> Result<Vec<Finding>, String> {
    let validated = tm(&["validate"], project);
    if !validated.status.success() {
        return Err(format!(
            "tm validate failed:\n{}{}",
            said(&validated.stdout),
            said(&validated.stderr)
        ));
    }
    let checked = tm(&["check", "--format", "json"], project);
    let report: Report = serde_json::from_slice(&checked.stdout).map_err(|error| {
        format!(
            "tm check printed no report ({error}):\n{}",
            said(&checked.stderr)
        )
    })?;
    let expected_code = if report.findings.is_empty() {
        CLEAN
    } else {
        FOUND
    };
    if checked.status.code() != Some(expected_code) {
        return Err(format!(
            "tm check exited {:?} with {} finding(s):\n{}",
            checked.status.code(),
            report.findings.len(),
            said(&checked.stderr)
        ));
    }
    let prefix = repository_prefix(project)?;
    let mut findings: Vec<Finding> = report
        .findings
        .into_iter()
        .map(|found| Finding {
            path: found
                .path
                .strip_prefix(&prefix)
                .map_or(found.path.clone(), str::to_owned),
            rule: found.rule,
            line: found.line,
        })
        .collect();
    findings.sort();
    Ok(findings)
}

/// A program's output as text.
pub(crate) fn said(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The path of `project` from the root of its repository, with a trailing `/`; empty at the root. Or why git could not say.
pub(crate) fn repository_prefix(project: &Path) -> Result<String, String> {
    let asked = Command::new("git")
        .args(["rev-parse", "--show-prefix"])
        .current_dir(project)
        .output()
        .map_err(|error| format!("git rev-parse did not start: {error}"))?;
    if !asked.status.success() {
        return Err(format!(
            "git rev-parse --show-prefix failed in {}:\n{}",
            project.display(),
            said(&asked.stderr)
        ));
    }
    Ok(said(&asked.stdout).trim().to_owned())
}

/// What a file held before an edit: its bytes, or nothing when the edit wrote it.
pub(crate) type Saved = (PathBuf, Option<Vec<u8>>);

/// Applies a row's edits in `project`, answering what each file held before; or why an edit could not be applied, with the
/// files put back.
pub(crate) fn apply(project: &Path, row: &Row) -> Result<Vec<Saved>, String> {
    let mut saved: Vec<Saved> = Vec::new();
    for edit in &row.edits {
        let path = project.join(&edit.path);
        let before = std::fs::read(&path).ok();
        if !saved.iter().any(|(kept, _)| *kept == path) {
            saved.push((path.clone(), before.clone()));
        }
        if let Err(why) = edited(&path, edit, before) {
            restore(&saved);
            return Err(why);
        }
    }
    Ok(saved)
}

/// Makes one edit of `path`, which held `before`.
pub(crate) fn edited(path: &Path, edit: &Edit, before: Option<Vec<u8>>) -> Result<(), String> {
    let shown = path.display();
    let written = match (
        edit.delete,
        edit.find.as_deref(),
        edit.replace.as_deref(),
        before,
    ) {
        (true, None, None, Some(_)) => {
            return std::fs::remove_file(path)
                .map_err(|error| format!("{shown} cannot be removed: {error}"));
        }
        (false, Some(find), Some(replace), Some(bytes)) => {
            let text = said(&bytes);
            if text.matches(find).count() != 1 {
                return Err(format!("`{find}` is not written once in {shown}"));
            }
            text.replacen(find, replace, 1)
        }
        (false, None, Some(replace), None) => replace.to_owned(),
        (_, _, _, before) => {
            return Err(format!(
                "{shown}: an edit that is no replacement, new file or deletion, or a file {}",
                if before.is_some() {
                    "there already"
                } else {
                    "not there"
                }
            ));
        }
    };
    path.parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(path, written))
        .map_err(|error| format!("{shown} cannot be written: {error}"))
}

/// Puts back what each edited file held, removing the files an edit wrote.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be restored is a broken test, and the message names which"
)]
pub(crate) fn restore(saved: &[Saved]) {
    for (path, before) in saved.iter().rev() {
        match before {
            Some(bytes) => std::fs::write(path, bytes).expect("the file restored"),
            None => std::fs::remove_file(path).expect("the written file removed"),
        }
    }
}

/// The samples that read, and a failure for each that does not.
pub(crate) fn readable(failures: &mut Vec<String>) -> Vec<Sample> {
    samples()
        .into_iter()
        .filter_map(|read| read.map_err(|why| failures.push(why)).ok())
        .collect()
}
