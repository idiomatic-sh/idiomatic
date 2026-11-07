//! The disk adapter's own plain types: a directory it binds, and a project it makes.

use crate::domain::files::types::{DirectoryName, MachinePath, SkippedDirectory};
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::types::Walk;

/// A directory of the disk, bound to its root: the files beneath it, walked as the adapter that bound it walks.
#[derive(Debug)]
pub(crate) struct RepositoryDiskRepository {
    /// Its root on the disk.
    pub root: MachinePath,
    /// The directories a walk never goes into, at any depth, from the linked languages.
    pub skipped: Vec<SkippedDirectory>,
    /// The store's directory, never walked.
    pub store: DirectoryName,
    /// Where the nearest `tastemaker.toml` is, when the directory was found by it; its root otherwise.
    pub workspace: crate::domain::paths::types::RepoPath,
    /// Where the search for it started, when it was found; its root otherwise.
    pub start: crate::domain::paths::types::RepoPath,
}

/// A project directory of the disk, bound to its root, removed with everything in it when it is dropped.
#[derive(Debug)]
pub(crate) struct RepositoryDiskProject {
    /// The directory, bound.
    pub repository: RepositoryDiskRepository,
}

impl Drop for RepositoryDiskProject {
    fn drop(&mut self) {
        // A directory already gone leaves nothing to remove.
        drop(std::fs::remove_dir_all(&*self.repository.root.0));
    }
}

/// One walker thread's files and its first error, handed to the shared walk and failure when the thread's visitor is
/// dropped.
pub(super) struct Kept<'a> {
    /// The files and directories this thread has found.
    pub walk: Walk,
    /// The first error this thread met.
    pub failure: Option<RepositoryError>,
    /// The walk shared by every thread, which receives `walk`.
    pub into: &'a std::sync::Mutex<Walk>,
    /// The failure shared by every thread, which receives `failure` if it holds none.
    pub failed: &'a std::sync::Mutex<Option<RepositoryError>>,
}

impl Drop for Kept<'_> {
    fn drop(&mut self) {
        if let Ok(mut shared) = self.into.lock() {
            shared.files.append(&mut self.walk.files);
            shared.directories.append(&mut self.walk.directories);
            shared.links.append(&mut self.walk.links);
        }
        if let Ok(mut failed) = self.failed.lock()
            && failed.is_none()
        {
            *failed = self.failure.take();
        }
    }
}
