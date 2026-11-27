//! Initialization requests and reports.
use tastemaker_values::domain::files::types::MachinePath;
use tastemaker_values::domain::paths::types::RepoPath;
use tastemaker_values::domain::writing::types::Writing;
/// The existing project to initialize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitRequest {
    /// The directory receiving its workspace and package.
    pub target: MachinePath,
    /// Whether files are written.
    pub writing: Writing,
}
/// A name as it is written inside a TOML basic string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StringBody(pub String);

/// What a path at a project's top is, which the entry `init` writes for it follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TopKind {
    /// A file: its entry names it.
    File,
    /// A directory: its entry places every file beneath it.
    Directory,
}

/// Files initialized, relative to the requested project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitReport {
    /// Files written, or planned by a dry run.
    pub written: Vec<RepoPath>,
}
