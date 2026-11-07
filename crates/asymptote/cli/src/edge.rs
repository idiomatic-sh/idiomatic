//! The edge every command takes: what the command line read of where it runs, built once before the command runs and
//! handed to it, so no command reads the process's directory, its build or its own format and dry-run switches.

/// What every command reads of where it runs: where the search for the project starts, where the command line was
/// started, where the store is kept, the build that runs, the format the answer is printed in, and whether the command
/// writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Where the project is looked for: this directory, then its ancestors.
    pub start: asymptote_core::domain::manifest::types::ProductRoot,
    /// The directory the command line was started in, before any `--root` replaced `start`.
    pub here: asymptote_core::domain::manifest::types::ProductRoot,
    /// The directory beneath a project's root where the store is kept.
    pub store: std::path::PathBuf,
    /// The build that runs.
    pub build: asymptote_core::outbound::check_store::types::Build,
    /// The format the answer is printed in: `--format`, `--json` or `-q`, taken anywhere on the line.
    pub format: crate::output::Format,
    /// Whether a command writes, or `-n/--dry-run` only says what it would write.
    pub writing: asymptote_core::domain::ratchet::types::Writing,
}
