//! A scratch directory the `repository` fake makes: a repository of its own, forgotten when it is dropped.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::paths::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::Repository;
use crate::outbound::repository::types::{Presence, RepoPaths, Walk};

/// A scratch directory of the fake, bound to its root: its files are forgotten when it is dropped.
#[derive(Debug)]
pub struct ProjectFake {
    /// The fake, bound to the scratch directory.
    pub repository: Box<dyn Repository>,
    /// The files of every scratch directory the fake holds, by directory: this one's are forgotten on drop.
    pub scratches: Arc<Mutex<BTreeMap<MachinePath, BTreeMap<RepoPath, FileText>>>>,
    /// The scratch directory.
    pub root: MachinePath,
}

impl Repository for ProjectFake {
    fn root(&self) -> &MachinePath {
        self.repository.root()
    }

    fn list(&self, under: &RepoPath) -> Result<RepoPaths, RepositoryError> {
        self.repository.list(under)
    }

    fn walk(&self, under: &RepoPath) -> Result<Walk, RepositoryError> {
        self.repository.walk(under)
    }

    fn read(&self, path: &RepoPath) -> Result<FileText, RepositoryError> {
        self.repository.read(path)
    }

    fn exists(&self, path: &RepoPath) -> Presence {
        self.repository.exists(path)
    }

    fn write(&self, path: &RepoPath, text: &FileText) -> Result<(), RepositoryError> {
        self.repository.write(path, text)
    }

    fn remove(&self, path: &RepoPath) -> Result<(), RepositoryError> {
        self.repository.remove(path)
    }

    fn stamps(&self, paths: &[RepoPath]) -> crate::domain::files::types::Stamps {
        self.repository.stamps(paths)
    }
}

impl Drop for ProjectFake {
    fn drop(&mut self) {
        drop(
            self.scratches
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&self.root),
        );
    }
}
