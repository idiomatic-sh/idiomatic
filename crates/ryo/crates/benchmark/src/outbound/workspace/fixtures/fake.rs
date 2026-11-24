//! In-memory workspace lifecycle for orchestration tests.
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use crate::outbound::workspace::types::{AppliedPatch, CommitRevision, TrackedFiles};
use benchmark_contracts::domain::reports::errors::ReportsError as Error;
use benchmark_contracts::domain::workloads::types::Project;
use std::path::{Path, PathBuf};
#[derive(Clone, Copy, Debug)]
/// Deterministic workspace substitute for orchestration and contract tests.
pub struct WorkspaceFake;
impl WorkspacePort for WorkspaceFake {
    fn prepare(&self, project: &Project) -> Result<Workspace, Error> {
        if project.name.0.is_empty()
            || project.revision.0.len() != 40
            || !project
                .revision
                .0
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::Invalid("invalid pin".into()));
        }
        Ok(Workspace {
            root: PathBuf::from(&project.name.0),
            revision: CommitRevision(project.revision.0.clone()),
            applied: None,
        })
    }
    fn tracked_files(&self, _: &Workspace) -> Result<TrackedFiles, Error> {
        Ok(TrackedFiles(vec!["input.txt".into()]))
    }
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error> {
        if workspace.applied.is_some() {
            return Err(Error::Invalid("already edited".into()));
        }
        workspace.applied = Some(AppliedPatch(patch.as_os_str().as_encoded_bytes().to_vec()));
        Ok(())
    }
    fn reset(&self, workspace: &mut Workspace) -> Result<(), Error> {
        workspace.applied = None;
        Ok(())
    }
}
#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;

/// A checkout of one project at a fixed directory, tracking the files it names.
#[derive(Debug)]
pub struct ProjectWorkspace {
    /// The checkout's root.
    pub root: PathBuf,
    /// The files the project tracks.
    pub tracked: Vec<String>,
}

impl WorkspacePort for ProjectWorkspace {
    fn prepare(&self, project: &Project) -> Result<Workspace, Error> {
        Ok(Workspace {
            root: self.root.clone(),
            revision: CommitRevision(project.revision.0.clone()),
            applied: None,
        })
    }
    fn tracked_files(&self, _: &Workspace) -> Result<TrackedFiles, Error> {
        Ok(TrackedFiles(self.tracked.clone()))
    }
    fn apply_patch(&self, _: &mut Workspace, _: &Path) -> Result<(), Error> {
        Ok(())
    }
    fn reset(&self, _: &mut Workspace) -> Result<(), Error> {
        Ok(())
    }
}
