//! Owned detached clones; restore only changes applied by the current handle.
use super::{
    constants,
    types::{GitArguments, GitInput, GitOutput},
};
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use crate::outbound::workspace::types::{AppliedPatch, CommitRevision, TrackedFiles};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub(crate) struct WorkspaceGitAdapter {
    settings: super::settings::WorkspaceGitSettings,
}
impl WorkspaceGitAdapter {
    pub(crate) fn new(settings: super::settings::WorkspaceGitSettings) -> Self {
        Self { settings }
    }
}

fn git(
    root: &Path,
    GitArguments(args): GitArguments<'_>,
    GitInput(input): GitInput<'_>,
) -> Result<GitOutput, Error> {
    let mut command = Command::new(constants::GIT);
    let _ = command
        .args([
            constants::CONFIG,
            constants::DISABLE_HOOKS,
            constants::CONFIG,
            constants::LOCAL_TRANSPORT,
        ])
        .args(args)
        .current_dir(root)
        .env(constants::PROMPT, constants::NO_PROMPT)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        let _ = command.stdin(Stdio::piped());
    }
    let mut child = command.spawn()?;
    if let Some(bytes) = input {
        child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid(constants::MISSING_STDIN.into()))?
            .write_all(bytes)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Invalid(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(GitOutput(output.stdout))
}
fn clean(root: &Path) -> Result<(), Error> {
    if !git(
        root,
        GitArguments(&[
            constants::STATUS,
            constants::PORCELAIN,
            constants::UNTRACKED,
        ]),
        GitInput(None),
    )?
    .0
    .is_empty()
    {
        return Err(Error::Invalid(constants::DIRTY.into()));
    }
    Ok(())
}
fn head(workspace: &Workspace) -> Result<(), Error> {
    let actual = git(
        &workspace.root,
        GitArguments(&[constants::REV_PARSE, constants::HEAD]),
        GitInput(None),
    )?
    .0;
    if String::from_utf8_lossy(&actual).trim() != workspace.revision.0 {
        return Err(Error::Invalid(constants::HEAD_CHANGED.into()));
    }
    Ok(())
}
impl WorkspacePort for WorkspaceGitAdapter {
    fn prepare(&self, project: &Project) -> Result<Workspace, Error> {
        if project.name.is_empty()
            || !project
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || project.revision.len() != 40
            || !project.revision.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Invalid(constants::UNPINNED.into()));
        }
        let cache = &self.settings.cache;
        fs::create_dir_all(cache)?;
        let root = cache.join(format!("{}-{}", project.name, project.revision));
        let absent = match fs::metadata(&root) {
            Ok(_) => false,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };
        if absent {
            fs::create_dir(&root)?;
            drop(git(
                &root,
                GitArguments(&[constants::INIT, constants::QUIET]),
                GitInput(None),
            )?);
            drop(git(
                &root,
                GitArguments(&[
                    constants::REMOTE,
                    constants::ADD,
                    constants::ORIGIN,
                    &project.repository,
                ]),
                GitInput(None),
            )?);
            drop(git(
                &root,
                GitArguments(&[
                    constants::FETCH,
                    constants::QUIET,
                    constants::SHALLOW,
                    constants::ORIGIN,
                    &project.revision,
                ]),
                GitInput(None),
            )?);
            drop(git(
                &root,
                GitArguments(&[
                    constants::CHECKOUT,
                    constants::QUIET,
                    constants::DETACH,
                    &project.revision,
                ]),
                GitInput(None),
            )?);
            fs::write(root.join(constants::OWNER), project.repository.as_bytes())?;
        }
        let marker = fs::read(root.join(constants::OWNER))?;
        if marker != project.repository.as_bytes() {
            return Err(Error::Invalid(constants::UNOWNED.into()));
        }
        let remote = git(
            &root,
            GitArguments(&[constants::REMOTE, constants::GET_URL, constants::ORIGIN]),
            GitInput(None),
        )?
        .0;
        if String::from_utf8_lossy(&remote).trim() != project.repository {
            return Err(Error::Invalid(constants::REMOTE_CHANGED.into()));
        }
        let workspace = Workspace {
            root,
            revision: CommitRevision(project.revision.clone()),
            applied: None,
        };
        head(&workspace)?;
        clean(&workspace.root)?;
        Ok(workspace)
    }
    fn tracked_files(&self, workspace: &Workspace) -> Result<TrackedFiles, Error> {
        head(workspace)?;
        let raw = git(
            &workspace.root,
            GitArguments(&[constants::LS_FILES, constants::NUL_PATHS]),
            GitInput(None),
        )?
        .0;
        let text =
            String::from_utf8(raw).map_err(|_| Error::Invalid(constants::NON_UTF8.into()))?;
        let mut files: Vec<String> = text
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(Error::Invalid(constants::EMPTY.into()));
        }
        Ok(TrackedFiles(files))
    }
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error> {
        if workspace.applied.is_some() {
            return Err(Error::Invalid(constants::ALREADY_PATCHED.into()));
        }
        head(workspace)?;
        clean(&workspace.root)?;
        let bytes = fs::read(patch)?;
        if !git(
            &workspace.root,
            GitArguments(&[constants::APPLY, constants::SUMMARY, constants::STDIN]),
            GitInput(Some(&bytes)),
        )?
        .0
        .is_empty()
            || git(
                &workspace.root,
                GitArguments(&[constants::APPLY, constants::NUMSTAT, constants::STDIN]),
                GitInput(Some(&bytes)),
            )?
            .0
            .is_empty()
        {
            return Err(Error::Invalid(constants::INVALID_PATCH.into()));
        }
        drop(git(
            &workspace.root,
            GitArguments(&[constants::APPLY, constants::CHECK, constants::STDIN]),
            GitInput(Some(&bytes)),
        )?);
        drop(git(
            &workspace.root,
            GitArguments(&[constants::APPLY, constants::STDIN]),
            GitInput(Some(&bytes)),
        )?);
        let diff = git(
            &workspace.root,
            GitArguments(&[constants::DIFF, constants::BINARY, constants::HEAD]),
            GitInput(None),
        )?
        .0;
        if diff.is_empty() {
            return Err(Error::Invalid(constants::NO_CHANGE.into()));
        }
        workspace.applied = Some(AppliedPatch(diff));
        Ok(())
    }
    fn reset(&self, workspace: &mut Workspace) -> Result<(), Error> {
        head(workspace)?;
        if let Some(applied) = &workspace.applied {
            let current = git(
                &workspace.root,
                GitArguments(&[constants::DIFF, constants::BINARY, constants::HEAD]),
                GitInput(None),
            )?
            .0;
            if &current != &applied.0 {
                return Err(Error::Invalid(constants::EDITED.into()));
            }
            drop(git(
                &workspace.root,
                GitArguments(&[
                    constants::APPLY,
                    constants::REVERSE,
                    constants::CHECK,
                    constants::STDIN,
                ]),
                GitInput(Some(&applied.0)),
            )?);
            drop(git(
                &workspace.root,
                GitArguments(&[constants::APPLY, constants::REVERSE, constants::STDIN]),
                GitInput(Some(&applied.0)),
            )?);
            workspace.applied = None;
        }
        clean(&workspace.root)
    }
}
