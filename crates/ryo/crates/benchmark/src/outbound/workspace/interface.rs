//! Reproducible corpus worktrees, owned exclusively by the benchmark.
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) revision: String,
    pub(crate) applied: Option<Vec<u8>>,
}
impl Workspace {
    pub fn root(&self) -> &Path {
        &self.root
    }
}
pub trait WorkspacePort {
    fn prepare(&self, project: &Project, cache: &Path) -> Result<Workspace, Error>;
    fn tracked_files(&self, workspace: &Workspace) -> Result<Vec<String>, Error>;
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error>;
    fn reset(&self, workspace: &mut Workspace) -> Result<(), Error>;
}
