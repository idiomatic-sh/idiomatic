//! In-memory workspace lifecycle for orchestration tests.
use crate::domain::{diagnostics::errors::DiagnosticsError as Error, suite::types::Project};
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use crate::outbound::workspace::types::{AppliedPatch, CommitRevision, TrackedFiles};
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
