//! The test data of the commands: a product on disk, in a temporary directory of its own, whose one tool `lint` prints
//! a SARIF log of one violation, at line 1, column 5 of `src/a.rs`. Compiled only for tests and under the `fixtures`
//! feature.
#![cfg(any(test, feature = "fixtures"))]

/// The tool's log: rule `r`, `a marker`, at `src/a.rs:1:5`.
const LOG: &str = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "lint", "rules": [{"id": "r"}]}}, "results": [{"ruleId": "r", "level": "warning", "message": {"text": "a marker"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/a.rs"}, "region": {"startLine": 1, "startColumn": 5}}}]}]}]}"#;

/// The same log, the violation suppressed by an in-source ignore.
const IGNORED: &str = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "lint", "rules": [{"id": "r"}]}}, "results": [{"ruleId": "r", "level": "warning", "message": {"text": "a marker"}, "suppressions": [{"kind": "inSource"}], "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/a.rs"}, "region": {"startLine": 1, "startColumn": 5}}}]}]}]}"#;

/// The product's `asymptote.toml`: its one SARIF tool, `lint`.
const MANIFEST: &str = "[[tools]]\nid = \"lint\"\nrun = [\"sh\", \"-c\", \"cat lint.sarif\"]\ngroup = \"lint\"\noutput = \"sarif\"\n";
/// The same manifest, with the `pre-commit` hook mapped to the `lint` group, as the commit gate is.
const HOOKED: &str = "[[tools]]\nid = \"lint\"\nrun = [\"sh\", \"-c\", \"cat lint.sarif\"]\ngroup = \"lint\"\noutput = \"sarif\"\n\n[hooks]\npre-commit = [\"lint\"]\n";

/// A product in a temporary directory: its manifest, the tool's log and the file it reports on.
///
/// # Errors
///
/// When the directory cannot be written.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn product() -> std::io::Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    for (path, text) in [
        ("asymptote.toml", HOOKED),
        ("lint.sarif", LOG),
        ("src/a.rs", "fn a() {}\n"),
    ] {
        let place = directory.path().join(path);
        if let Some(parent) = place.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(place, text)?;
    }
    Ok(directory)
}

/// The product, its one violation suppressed by an in-source ignore.
///
/// # Errors
///
/// When the directory cannot be written.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn ignoring() -> std::io::Result<tempfile::TempDir> {
    let directory = product()?;
    std::fs::write(directory.path().join("lint.sarif"), IGNORED)?;
    Ok(directory)
}

/// The edge of a command run in `directory`.
#[must_use]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn edge(directory: &tempfile::TempDir) -> crate::edge::Edge {
    let root = asymptote_core::domain::manifest::types::ProductRoot(
        directory.path().display().to_string().into(),
    );
    crate::edge::Edge {
        here: root.clone(),
        start: root,
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build("test".into()),
        format: crate::output::Format::Terminal,
        writing: asymptote_core::domain::ratchet::types::Writing::Write,
    }
}

/// `edge` with the format and the writing a row's command line chose, as the command tree reads them from the line.
#[must_use]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn chosen(
    edge: crate::edge::Edge,
    output: crate::output::FormatSwitch,
    dry_run: crate::output::dry_run::DryRunSwitch,
) -> crate::edge::Edge {
    crate::edge::Edge {
        format: output.format(),
        writing: if dry_run.dry_run() {
            asymptote_core::domain::ratchet::types::Writing::DryRun
        } else {
            asymptote_core::domain::ratchet::types::Writing::Write
        },
        ..edge
    }
}

/// The tool's log when it finds nothing.
const CLEAN: &str = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "lint", "rules": [{"id": "r"}]}}, "results": []}]}"#;

/// What a workspace root is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) enum Members {
    /// Two members, `crates/a` with the violation and `crates/b` clean, and no tool of the root's own.
    Two,
    /// The same, and a tool of the root's own that finds nothing.
    TwoAndRoot,
    /// The same as `Two`, and a directory `crates/c` with no `asymptote.toml`.
    OneWithoutManifest,
    /// The same as `Two`, the root's `[hooks]` mapping `pre-commit` to a group no tool declares.
    UndeclaredHook,
}

/// A workspace in a temporary directory: its root `asymptote.toml` names `crates/*`.
///
/// # Errors
///
/// When the directory cannot be written.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn workspace(members: Members) -> std::io::Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    let root = match members {
        Members::TwoAndRoot => format!("members = [\"crates/*\"]\n\n{MANIFEST}"),
        Members::UndeclaredHook => {
            "members = [\"crates/*\"]\n\n[hooks]\npre-commit = [\"typo\"]\n".to_owned()
        }
        Members::Two | Members::OneWithoutManifest => "members = [\"crates/*\"]\n".to_owned(),
    };
    let mut files = vec![
        ("asymptote.toml", root.as_str()),
        ("lint.sarif", CLEAN),
        ("crates/a/asymptote.toml", MANIFEST),
        ("crates/a/lint.sarif", LOG),
        ("crates/a/src/a.rs", "fn a() {}\n"),
        ("crates/b/asymptote.toml", MANIFEST),
        ("crates/b/lint.sarif", CLEAN),
    ];
    if members == Members::OneWithoutManifest {
        files.push(("crates/c/src/c.rs", "fn c() {}\n"));
    }
    for (path, text) in files {
        let place = directory.path().join(path);
        if let Some(parent) = place.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(place, text)?;
    }
    Ok(directory)
}

/// The perf product's `asymptote.toml`: `bench`, in the `perf` group, reading Bencher Metric Format, and `noop`, an
/// exit-status tool of the `lint` group that always passes.
const PERF: &str = "[[tools]]\nid = \"bench\"\nrun = [\"sh\", \"-c\", \"cat bench.json\"]\ngroup = \"perf\"\noutput = \"bmf\"\n\n[[tools]]\nid = \"noop\"\nrun = [\"true\"]\ngroup = \"lint\"\n";

/// A product in a temporary directory whose one tool, `bench`, measures `wall-ms` of benchmark `check` at 100.
///
/// # Errors
///
/// When the directory cannot be written.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fixtures feature builds the test data no build calls"
    )
)]
pub(crate) fn measured() -> std::io::Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("asymptote.toml"), PERF)?;
    std::fs::write(
        directory.path().join("bench.json"),
        r#"{"check": {"wall-ms": {"value": 100}}}"#,
    )?;
    Ok(directory)
}
