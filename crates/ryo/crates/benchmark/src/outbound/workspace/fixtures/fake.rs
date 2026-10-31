//! In-memory workspace lifecycle for orchestration tests.
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use std::path::Path;
#[derive(Clone, Copy, Debug)]
/// Deterministic workspace substitute for orchestration and contract tests.
pub struct WorkspaceFake;
impl WorkspacePort for WorkspaceFake {
    fn prepare(&self, project: &Project, cache: &Path) -> Result<Workspace, Error> {
        if project.name.is_empty()
            || project.revision.len() != 40
            || !project
                .revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(Error::Invalid("invalid pin".into()));
        }
        Ok(Workspace {
            root: cache.join(&project.name),
            revision: project.revision.clone(),
            applied: None,
        })
    }
    fn tracked_files(&self, _: &Workspace) -> Result<Vec<String>, Error> {
        Ok(vec!["input.txt".into()])
    }
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error> {
        if workspace.applied.is_some() {
            return Err(Error::Invalid("already edited".into()));
        }
        workspace.applied = Some(patch.as_os_str().as_encoded_bytes().to_vec());
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
