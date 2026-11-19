//! The bench's proofs, run before it measures anything and alone under `cargo test`: the target's
//! preparation keeps declared dependencies as isolated copies and lets no path or symlink escape
//! the target or the repository; a summary is independent of order and names every change; and
//! the findings guard refuses another exit code or other findings than the target states.

use super::project::{hold, preparation, summary};
use std::path::Path;

/// `holds`, or an error saying what does not.
fn ensure(holds: bool, what: impl FnOnce() -> String) -> std::io::Result<()> {
    if holds {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("a proof fails: {}", what())))
    }
}

/// A pin whose one dependency binds `source`, in the repository, to `destination`, in the project.
fn pin(source: &str, destination: &str) -> toml::Table {
    toml::toml! { dependencies = [{ source = source, destination = destination }] }
}

/// A declared directory is an isolated external copy, a declared file is copied in, and no
/// source or destination may leave its root, be empty or be bound twice.
fn dependencies_are_isolated_external_packages_and_paths_cannot_escape() -> std::io::Result<()> {
    let scratch = tempfile::tempdir()?;
    let repository = scratch.path().join("repository");
    let project = scratch.path().join("target");
    let external = scratch.path().join("dependencies");
    std::fs::create_dir_all(repository.join("package"))?;
    std::fs::create_dir(&project)?;
    std::fs::write(repository.join("package/spec.toml"), "before")?;
    preparation::dependencies(
        &repository,
        &project,
        &external,
        &pin("package", ".benchmark/arch"),
    )?;
    let binding = project.join(".benchmark/arch");
    // Compared as canonical paths both sides: the temporary directory may be reached through a symbolic link (macOS's
    // /var is /private/var).
    let bound = binding.canonicalize()?;
    ensure(
        !bound.starts_with(project.canonicalize()?)
            && bound.starts_with(scratch.path().canonicalize()?),
        || format!("the binding {} is not an external copy", bound.display()),
    )?;
    std::fs::write(repository.join("package/spec.toml"), "after")?;
    ensure(
        std::fs::read_to_string(binding.join("spec.toml"))? == "before",
        || "the binding follows its source".to_owned(),
    )?;
    std::fs::write(repository.join("configuration.lock"), "locked")?;
    preparation::dependencies(
        &repository,
        &project,
        &external,
        &pin("configuration.lock", "configuration.lock"),
    )?;
    ensure(
        std::fs::read_to_string(project.join("configuration.lock"))? == "locked",
        || "a declared file is not copied in".to_owned(),
    )?;
    for bad in ["../outside", "/outside", "", "a/../../outside"] {
        ensure(
            preparation::dependencies(&repository, &project, &external, &pin(bad, "copy")).is_err(),
            || format!("the source `{bad}` is taken"),
        )?;
        ensure(
            preparation::dependencies(&repository, &project, &external, &pin("package", bad))
                .is_err(),
            || format!("the destination `{bad}` is taken"),
        )?;
    }
    ensure(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &pin("package", ".benchmark/arch"),
        )
        .is_err(),
        || "a destination is bound twice".to_owned(),
    )?;
    ensure(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &toml::toml! {dependencies = "wrong"},
        )
        .is_err(),
        || "dependencies that are not a list are taken".to_owned(),
    )
}

/// No symlink in a source or a destination leads out of the repository or the target.
#[cfg(unix)]
fn dependency_source_and_destination_symlinks_cannot_escape() -> std::io::Result<()> {
    let scratch = tempfile::tempdir()?;
    let repository = scratch.path().join("repository");
    let project = scratch.path().join("target");
    let external = scratch.path().join("dependencies");
    std::fs::create_dir_all(repository.join("package"))?;
    std::fs::create_dir(&project)?;
    std::os::unix::fs::symlink(scratch.path(), repository.join("escape"))?;
    std::os::unix::fs::symlink(scratch.path(), project.join("escape"))?;
    ensure(
        preparation::dependencies(&repository, &project, &external, &pin("escape", "copy"))
            .is_err(),
        || "a symlinked source is followed".to_owned(),
    )?;
    ensure(
        preparation::dependencies(
            &repository,
            &project,
            &external,
            &pin("package", "escape/copied"),
        )
        .is_err(),
        || "a symlinked destination is followed".to_owned(),
    )?;
    std::os::unix::fs::symlink(scratch.path(), repository.join("package/escape"))?;
    ensure(
        preparation::dependencies(&repository, &project, &external, &pin("package", "copy"))
            .is_err(),
        || "a symlink inside a source is copied".to_owned(),
    )
}

/// Off POSIX no symlink is made, so there is nothing to escape through.
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "every proof shares one signature, through which `check` runs them"
)]
const fn dependency_source_and_destination_symlinks_cannot_escape() -> std::io::Result<()> {
    Ok(())
}

/// A summary is the same for the same findings in any order, names each rule's change, ignores
/// only a tool's boolean `reused`, reads back as itself, and refuses a stored total or a missing
/// exit code.
fn a_summary_is_independent_of_order_and_names_every_change() -> std::io::Result<()> {
    let first = br#"{"passed":false,"findings":[{"rule":"a","path":"x"},{"rule":"b","path":"y"}]}"#;
    let reordered =
        br#"{"findings":[{"path":"y","rule":"b"},{"path":"x","rule":"a"}],"passed":false}"#;
    let changed =
        br#"{"passed":false,"findings":[{"rule":"a","path":"x"},{"rule":"a","path":"z"}]}"#;
    let summary = summary::of(first, 1)?;
    ensure(summary::of(reordered, 1)?.matches(&summary), || {
        "the order of the findings changes the summary".to_owned()
    })?;
    let other = summary::of(changed, 1)?;
    ensure(!other.matches(&summary), || {
        "other findings match the summary".to_owned()
    })?;
    let changes = other.changes(&summary);
    ensure(
        changes == ["a 1 → 2 (+1)", "b 1 → 0 (-1)", "total 2 → 2 (+0)"],
        || format!("the changes are {changes:?}"),
    )?;
    let mut cached = serde_json::json!({
        "tools": [{"id": "tastemaker", "status": "completed", "reused": false, "error": null}],
        "violations": [{"rule": "a", "path": "x", "reused": false}]
    });
    let cold = summary::of(&serde_json::to_vec(&cached)?, 0)?;
    let reused = cached
        .pointer_mut("/tools/0/reused")
        .ok_or_else(|| std::io::Error::other("the document has no `reused`"))?;
    *reused = serde_json::json!(true);
    ensure(
        summary::of(&serde_json::to_vec(&cached)?, 0)? == cold,
        || "a reused tool changes the summary".to_owned(),
    )?;
    for (pointer, value) in [
        ("/tools/0/id", serde_json::json!("another")),
        ("/tools/0/status", serde_json::json!("failed")),
        ("/tools/0/error", serde_json::json!("tool failed")),
        ("/tools/0/reused", serde_json::json!("true")),
        ("/tools/0/reused", serde_json::Value::Null),
        ("/violations/0/path", serde_json::json!("y")),
        ("/violations/0/reused", serde_json::json!(true)),
    ] {
        let mut changed = cached.clone();
        let at = changed
            .pointer_mut(pointer)
            .ok_or_else(|| std::io::Error::other(format!("the document has no `{pointer}`")))?;
        *at = value;
        ensure(
            summary::of(&serde_json::to_vec(&changed)?, 0)? != cold,
            || format!("{pointer} does not change the summary"),
        )?;
    }
    let file = tempfile::NamedTempFile::new()?;
    std::fs::write(file.path(), summary.text())?;
    ensure(summary::read(file.path())? == summary, || {
        "a summary does not read back as itself".to_owned()
    })?;
    // The total is the counts' sum, never a stored key.
    std::fs::write(file.path(), format!("total = 2\n{}", summary.text()))?;
    ensure(summary::read(file.path()).is_err(), || {
        "a stored total is read".to_owned()
    })?;
    // A summary with no finding has no `[rules]` table, and reads back as itself.
    let empty = summary::of(br#"{"findings":[]}"#, 0)?;
    ensure(!empty.text().contains("[rules]"), || {
        "a summary with no finding has a `[rules]` table".to_owned()
    })?;
    std::fs::write(file.path(), empty.text())?;
    ensure(summary::read(file.path())? == empty, || {
        "an empty summary does not read back as itself".to_owned()
    })?;
    std::fs::write(file.path(), summary.text().replace("exit_code = 1\n", ""))?;
    ensure(summary::read(file.path()).is_err(), || {
        "a summary without its exit code is read".to_owned()
    })
}

/// The guard passes the findings and exit code a summary states, and refuses another exit code or
/// a findings document whose digest differs, naming what changed.
fn the_findings_guard_refuses_other_findings_or_exit_codes() -> std::io::Result<()> {
    let printed = br#"{"findings":[{"rule":"a","path":"x"},{"rule":"b","path":"y"}]}"#;
    let file = tempfile::NamedTempFile::new()?;
    let expected: &Path = file.path();
    std::fs::write(expected, summary::of(printed, 1)?.text())?;
    let total = hold(printed, Some(1), expected)?;
    ensure(total == 2, || format!("the guard counts {total} findings"))?;
    let ended = hold(printed, Some(0), expected).map_err(|error| error.to_string());
    ensure(
        ended
            .as_ref()
            .is_err_and(|error| error.contains("ended with exit code 0, not the `exit_code = 1`")),
        || format!("another exit code is held as {ended:?}"),
    )?;
    let other = hold(br#"{"findings":[]}"#, Some(1), expected).map_err(|error| error.to_string());
    ensure(
        other
            .as_ref()
            .is_err_and(|error| error.contains("its findings are not those of")),
        || format!("other findings are held as {other:?}"),
    )
}

/// Runs every proof, stopping at the first that fails.
pub(super) fn check() -> std::io::Result<()> {
    dependencies_are_isolated_external_packages_and_paths_cannot_escape()?;
    dependency_source_and_destination_symlinks_cannot_escape()?;
    a_summary_is_independent_of_order_and_names_every_change()?;
    the_findings_guard_refuses_other_findings_or_exit_codes()
}
