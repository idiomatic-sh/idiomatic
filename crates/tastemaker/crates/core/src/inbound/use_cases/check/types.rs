//! The types of the `check` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::MachinePath;
use crate::domain::findings::types::Report;
use crate::domain::paths::types::RepoPath;

/// Whether a check rewrites the lock when it is stale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fixing {
    /// A stale lock refuses the check.
    Report,
    /// A stale lock is rewritten, and the check goes on.
    Fix,
}

/// What `check` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckRequest {
    /// Where to look for the workspace: this directory, then its ancestors.
    pub start: MachinePath,
    /// Only the findings at or beneath these paths are reported; none means every finding.
    pub within: Vec<RepoPath>,
    /// Whether a stale lock is rewritten.
    pub fixing: Fixing,
    /// Whether the check writes its lock, or `-n/--dry-run` only says it would.
    pub writing: crate::domain::writing::types::Writing,
}

/// Whether the lock was rewritten.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockOutcome {
    /// It was current.
    Current,
    /// It was stale or missing, and was rewritten.
    Rewritten,
    /// It was stale or missing, and `-n/--dry-run` kept it as it was: it would be rewritten.
    Planned,
}

/// What `check` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckReport {
    /// What it found.
    pub report: Report,
    /// The repository's root on the machine, which a SARIF log's paths are relative to.
    pub repository: MachinePath,
    /// Whether the lock was rewritten.
    pub lock: LockOutcome,
}

/// Where the lint call reads: the repository's root, and the files of the corpus.
#[derive(Debug)]
pub struct LintInputs<'a> {
    /// The repository's root.
    pub root: &'a MachinePath,
    /// The files of the corpus.
    pub files: &'a [RepoPath],
}

/// The plan a run prepared, with what it was made from: the lock the load made, what loading found, and the definition
/// files it read.
#[derive(Debug)]
pub struct PreparedPlan {
    /// The plan.
    pub plan: crate::domain::applications::types::Plan,
    /// The lock the load made, its packages.
    pub lock: Vec<crate::domain::packages::types::LockedPackage>,
    /// What loading found: the format findings and the variables'.
    pub findings: Vec<crate::domain::findings::types::Finding>,
    /// The definition files read, each in its package.
    pub read: Vec<crate::domain::loading::types::ReadFile>,
}

/// What a walk of the workspace listed: every file, its symbolic links among them, and the links alone.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    /// The files, sorted.
    pub files: Vec<RepoPath>,
    /// The symbolic links, sorted.
    pub links: Vec<RepoPath>,
}
