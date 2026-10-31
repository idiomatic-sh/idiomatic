//! Reproducible corpus worktrees, owned exclusively by the benchmark.
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use std::path::{Path, PathBuf};

/// An owned checkout and the evidence needed to restore its current edit.
#[derive(Debug)]
pub struct Workspace {
    pub(crate) root: PathBuf,
    pub(crate) revision: String,
    pub(crate) applied: Option<Vec<u8>>,
}
impl Workspace {
    /// Root directory of this source snapshot.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}
/// Prepare and edit benchmark-owned source snapshots without discarding external changes.
pub trait WorkspacePort {
    /// Acquire a clean checkout of the requested immutable project revision.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid pins or an existing checkout with conflicting ownership or changes.
    fn prepare(&self, project: &Project, cache: &Path) -> Result<Workspace, Error>;
    /// List all tracked relative paths before language-specific input selection.
    ///
    /// # Errors
    ///
    /// Returns an error when repository state or path encoding cannot be verified.
    fn tracked_files(&self, workspace: &Workspace) -> Result<Vec<String>, Error>;
    /// Apply a checked edit to existing tracked files and retain restoration evidence.
    ///
    /// # Errors
    ///
    /// Returns an error when the edit cannot be applied safely or the workspace is already edited.
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error>;
    /// Restore only the edit applied by this workspace handle.
    ///
    /// # Errors
    ///
    /// Returns an error if intervening changes prevent a safe restoration.
    fn reset(&self, workspace: &mut Workspace) -> Result<(), Error>;
}
