//! The edge every command takes: what the command line read of where it runs, built once before the command runs and
//! handed to it, so no command reads the process's directory, its build or its own format switches.

/// What every command reads of where it runs: where the search for the project starts, where the command line was
/// started, where the store is kept, the build that runs and the format the answer is printed in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// The project's directory: where the command reads, with no search in its ancestors.
    pub start: ryo_language::inbound::use_cases::project::types::RootPath,
    /// The directory the command line was started in, before any `--root` replaced `start`.
    pub here: ryo_language::inbound::use_cases::project::types::RootPath,
    /// The directory beneath a project's root where the store is kept.
    pub store: ryo_core::outbound::lint_store::types::CacheDirectory,
    /// The build that runs.
    pub build: ryo_core::outbound::lint_store::types::BuildIdentity,
    /// The format the answer is printed in: `--format`, `--json` or `-q`, taken anywhere on the line.
    pub format: crate::output::Format,
}
