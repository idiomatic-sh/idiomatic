//! Owned detached clones; restore only changes applied by the current handle.
use crate::domain::{diagnostics::errors::Error, suite::types::Project};
use crate::outbound::workspace::interface::{Workspace, WorkspacePort};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub struct WorkspaceGitAdapter;

fn git(root: &Path, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, Error> {
    let mut command = Command::new("git");
    command
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "protocol.file.allow=always",
        ])
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn()?;
    if let Some(bytes) = input {
        child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid("git stdin missing".into()))?
            .write_all(bytes)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(Error::Invalid(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(output.stdout)
}
fn clean(root: &Path) -> Result<(), Error> {
    if !git(
        root,
        &["status", "--porcelain", "--untracked-files=all"],
        None,
    )?
    .is_empty()
    {
        return Err(Error::Invalid(
            "corpus is dirty; refusing to overwrite changes".into(),
        ));
    }
    Ok(())
}
fn head(workspace: &Workspace) -> Result<(), Error> {
    let actual = git(&workspace.root, &["rev-parse", "HEAD"], None)?;
    if String::from_utf8_lossy(&actual).trim() != workspace.revision {
        return Err(Error::Invalid("corpus HEAD changed".into()));
    }
    Ok(())
}
impl WorkspacePort for WorkspaceGitAdapter {
    fn prepare(&self, project: &Project, cache: &Path) -> Result<Workspace, Error> {
        if project.name.is_empty()
            || !project
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || project.revision.len() != 40
            || !project.revision.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::Invalid(
                "project requires a safe name and full commit revision".into(),
            ));
        }
        fs::create_dir_all(cache)?;
        let root = cache.join(format!("{}-{}", project.name, project.revision));
        if !root.exists() {
            fs::create_dir(&root)?;
            git(&root, &["init", "--quiet"], None)?;
            git(
                &root,
                &["remote", "add", "origin", &project.repository],
                None,
            )?;
            git(
                &root,
                &["fetch", "--quiet", "--depth=1", "origin", &project.revision],
                None,
            )?;
            git(
                &root,
                &["checkout", "--quiet", "--detach", &project.revision],
                None,
            )?;
            fs::write(
                root.join(".git/ryo-benchmark-owner"),
                project.repository.as_bytes(),
            )?;
        }
        let marker = fs::read(root.join(".git/ryo-benchmark-owner"))?;
        if marker != project.repository.as_bytes() {
            return Err(Error::Invalid("cache is not this benchmark's clone".into()));
        }
        let remote = git(&root, &["remote", "get-url", "origin"], None)?;
        if String::from_utf8_lossy(&remote).trim() != project.repository {
            return Err(Error::Invalid("corpus remote changed".into()));
        }
        let workspace = Workspace {
            root,
            revision: project.revision.clone(),
            applied: None,
        };
        head(&workspace)?;
        clean(&workspace.root)?;
        Ok(workspace)
    }
    fn tracked_files(&self, workspace: &Workspace) -> Result<Vec<String>, Error> {
        head(workspace)?;
        let raw = git(&workspace.root, &["ls-files", "-z"], None)?;
        let text =
            String::from_utf8(raw).map_err(|_| Error::Invalid("non-UTF8 corpus path".into()))?;
        let mut files: Vec<String> = text
            .split('\0')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(Error::Invalid("empty corpus".into()));
        }
        Ok(files)
    }
    fn apply_patch(&self, workspace: &mut Workspace, patch: &Path) -> Result<(), Error> {
        if workspace.applied.is_some() {
            return Err(Error::Invalid("patch already applied".into()));
        }
        head(workspace)?;
        clean(&workspace.root)?;
        let bytes = fs::read(patch)?;
        if !git(&workspace.root, &["apply", "--summary", "-"], Some(&bytes))?.is_empty()
            || git(&workspace.root, &["apply", "--numstat", "-"], Some(&bytes))?.is_empty()
        {
            return Err(Error::Invalid(
                "edit patch must change existing tracked files only".into(),
            ));
        }
        git(&workspace.root, &["apply", "--check", "-"], Some(&bytes))?;
        git(&workspace.root, &["apply", "-"], Some(&bytes))?;
        let diff = git(&workspace.root, &["diff", "--binary", "HEAD"], None)?;
        if diff.is_empty() {
            return Err(Error::Invalid("patch makes no tracked source edit".into()));
        }
        workspace.applied = Some(diff);
        Ok(())
    }
    fn reset(&self, workspace: &mut Workspace) -> Result<(), Error> {
        head(workspace)?;
        if let Some(applied) = &workspace.applied {
            let current = git(&workspace.root, &["diff", "--binary", "HEAD"], None)?;
            if &current != applied {
                return Err(Error::Invalid(
                    "corpus changed after patch; refusing restore".into(),
                ));
            }
            git(
                &workspace.root,
                &["apply", "--reverse", "--check", "-"],
                Some(applied),
            )?;
            git(&workspace.root, &["apply", "--reverse", "-"], Some(applied))?;
            workspace.applied = None;
        }
        clean(&workspace.root)
    }
}
