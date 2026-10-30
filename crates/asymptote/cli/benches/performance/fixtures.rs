//! The product every workload runs over: one tool that prints a log of one violation.

/// The tool's log: one warning of rule `r` at `src/a.rs:1:5`.
const LOG: &str = r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "lint", "rules": [{"id": "r"}]}}, "results": [{"ruleId": "r", "level": "warning", "message": {"text": "a marker"}, "locations": [{"physicalLocation": {"artifactLocation": {"uri": "src/a.rs"}, "region": {"startLine": 1, "startColumn": 5}}}]}]}]}"#;

/// A product in a temporary directory whose one tool `lint` prints that log, checked once, so its cache holds a
/// report and its run.
///
/// # Errors
///
/// When the directory cannot be written.
pub(crate) fn product() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let files = [
        (
            "asymptote.toml",
            "[[tools]]\nid = \"lint\"\nrun = [\"sh\", \"-c\", \"cat lint.sarif\"]\ngroup = \"lint\"\noutput = \"sarif\"\n",
        ),
        ("lint.sarif", LOG),
        ("src/a.rs", "fn a() {}\n"),
    ];
    for (path, text) in files {
        let place = directory.path().join(path);
        std::fs::create_dir_all(
            place
                .parent()
                .ok_or("a fixture file has no parent directory")?,
        )?;
        std::fs::write(place, text)?;
    }
    let edge = edge(&directory);
    let outcome = asymptote_cli::resources::ratchet::check::command::run(
        arguments(&["check"])?,
        &edge,
        |_| {},
    );
    if outcome != asymptote_cli::outcome::Outcome::Failure {
        return Err("expected: the benchmark product has one new violation".into());
    }
    Ok(directory)
}

/// A git repository in a temporary directory, with nothing in it.
///
/// # Errors
///
/// When the directory cannot be made or git cannot run.
pub(crate) fn repository() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let made = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(directory.path())
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()?;
    if !made.status.success() {
        return Err("expected: git initializes the repository".into());
    }
    Ok(directory)
}

/// The edge of a command run in `directory`, quietly: `-q`, which prints no answer.
pub(crate) fn edge(directory: &tempfile::TempDir) -> asymptote_cli::edge::Edge {
    let root = asymptote_core::domain::manifest::types::ProductRoot(
        directory.path().display().to_string().into(),
    );
    asymptote_cli::edge::Edge {
        here: root.clone(),
        start: root,
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build("bench".into()),
        format: asymptote_cli::output::Format::Quiet,
        writing: asymptote_core::domain::ratchet::types::Writing::Write,
    }
}

/// `edge` under `-n/--dry-run`: the command says what it would write and writes nothing.
pub(crate) fn dry(edge: asymptote_cli::edge::Edge) -> asymptote_cli::edge::Edge {
    asymptote_cli::edge::Edge {
        writing: asymptote_core::domain::ratchet::types::Writing::DryRun,
        ..edge
    }
}

/// A command's arguments, parsed from `line`, the command's name first.
///
/// # Errors
///
/// When the line does not parse.
pub(crate) fn arguments<A>(line: &[&str]) -> Result<A, Box<dyn std::error::Error>>
where
    A: clap::Args + clap::FromArgMatches,
{
    let command = A::augment_args(clap::Command::new("bench"));
    let matches = command.try_get_matches_from(line)?;
    Ok(A::from_arg_matches(&matches)?)
}
