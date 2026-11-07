//! What the `repository` port asks of the outside, in the core's own vocabulary: the repository a workspace is in, found
//! and bound to its root, whose files are then listed, read and written through the bound repository.

use std::sync::Arc;

use super::errors::RepositoryError;
use super::types::{Presence, RepoPaths, Walk};
use crate::domain::files::types::{FileText, MachinePath, SkippedDirectories, Stamps};
use crate::domain::paths::types::RepoPath;

/// What every `repository` adapter implements.
pub trait RepositoryPort: Send + Sync {
    /// The repository the workspace nearest at or above `start` is in, bound to its root.
    ///
    /// # Errors
    ///
    /// `NoWorkspace` when no directory at or above holds a `tastemaker.toml`, `NoRepository` when none holds `.git`.
    fn find(&self, start: &MachinePath) -> Result<Arc<dyn FoundRepository>, RepositoryError>;

    /// The git repository at or above `start`: the directory that holds `.git`.
    ///
    /// # Errors
    ///
    /// `NoRepository` when no directory at or above holds `.git`; `Read` when `start` cannot be resolved.
    fn repository(&self, start: &MachinePath) -> Result<MachinePath, RepositoryError>;

    /// The directory at `path`, opened as a repository bound to it: a directory a request names, such as the one `init`
    /// writes a workspace into, a package `create` writes into, or the one `explain` writes its pages into.
    fn open(&self, path: &MachinePath) -> Arc<dyn Repository>;

    /// The directory names a walk never goes into, at any depth, each with who declares it, in name order.
    fn skipped(&self) -> SkippedDirectories;

    /// A new, empty project at `root`, which the core chose, bound to it: a case's files are written into it, as its
    /// repository, and the adapter removes it, with everything in it, when it is dropped.
    ///
    /// # Errors
    ///
    /// `Write` when it cannot be made, or `root` is already there.
    fn project(&self, root: &MachinePath) -> Result<Box<dyn Repository>, RepositoryError>;
}

/// The repository a workspace was found in: bound to its root as every repository is, and saying where the nearest `tastemaker.toml`
/// is in it and where the search started. A directory opened by name has neither: both are its own root.
pub trait FoundRepository: Repository {
    /// The directory of the nearest `tastemaker.toml`, relative to the repository.
    fn workspace(&self) -> &RepoPath;

    /// Where the search started, relative to the repository.
    fn start(&self) -> &RepoPath;
}

/// A directory of the machine, bound to its root by the port that handed it out: every path it is asked about is
/// relative to that root, and no root is passed to it again.
pub trait Repository: Send + Sync + std::fmt::Debug {
    /// Its root on the machine.
    fn root(&self) -> &MachinePath;

    /// Every file beneath `under`, sorted: `.gitignore` applies, hidden files are listed, the directories the port skips
    /// not entered.
    ///
    /// # Errors
    ///
    /// `Read` when a directory cannot be listed.
    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError>;

    /// Every file beneath `under`, as `list` finds them, and every directory the walk went into.
    ///
    /// # Errors
    ///
    /// `Read` when a directory cannot be listed.
    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError>;

    /// A file's text.
    ///
    /// # Errors
    ///
    /// `Read` when it cannot be read or is not text.
    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError>;

    /// Whether a path exists.
    fn exists(&self, path: &RepoPath) -> Presence;

    /// Writes a file, making its directories.
    ///
    /// # Errors
    ///
    /// `Write` when it cannot be written.
    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError>;

    /// Removes a file, or a directory that holds nothing.
    ///
    /// # Errors
    ///
    /// `Write` when it cannot be removed: it is not there, or it is a directory that holds something.
    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError>;

    /// What each file's size and modification time come to, in order; an empty stamp for one that is gone.
    fn stamps(&self, paths: &[RepoPath]) -> Stamps;
}
