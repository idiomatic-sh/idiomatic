//! The hooks port, implemented with the git command line: where a hook is kept is what `git rev-parse --git-path`
//! answers, so `core.hooksPath` and linked worktrees hold, and the script is written there, executable.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[cfg(unix)]
use super::constants::EXECUTABLE;
use super::constants::{GIT_PATH, HOOKS};
use crate::domain::git::constants::{GIT, HOOK_VARIABLES, NOT_A_REPOSITORY, TOP_LEVEL};
use crate::domain::hooks::types::ScriptText;
use crate::domain::inputs::constants::SEPARATOR;
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::interface::HooksPort;
use crate::outbound::hooks::types::{GitAnswer, HookFile, Installed};

/// The git hooks adapter.
#[derive(Debug)]
pub(crate) struct HooksGitAdapter;

/// What git prints for `command`, run in the directory `root` with the hook variables removed.
fn printed(command: &mut Command, root: &ProductRoot) -> Result<GitAnswer, HooksError> {
    let output = HOOK_VARIABLES
        .iter()
        .fold(
            command
                .current_dir(Path::new(&*root.0))
                .stdin(Stdio::null()),
            |command, variable| command.env_remove(variable),
        )
        .output()
        .map_err(|error| HooksError::Git {
            reason: error.to_string(),
        })?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(if said.contains(NOT_A_REPOSITORY) {
            HooksError::NotARepository {
                path: root.0.to_string(),
            }
        } else {
            HooksError::Git {
                reason: said.trim().to_owned(),
            }
        });
    }
    Ok(GitAnswer(
        String::from_utf8_lossy(&output.stdout).trim().into(),
    ))
}

/// Where git keeps the hook `name` of the repository the product at `root` is in.
fn path_of(root: &ProductRoot, name: &HookName) -> Result<PathBuf, HooksError> {
    // Joined by `/` on every platform: git maps `hooks/<name>` to the common directory a linked worktree shares only
    // when it is written so.
    let mut hook = HOOKS.to_owned();
    hook.push(SEPARATOR);
    hook.push_str(&name.0);
    let answer = printed(Command::new(GIT).args(GIT_PATH).arg(hook), root)?;
    Ok(Path::new(&*root.0).join(&*answer.0))
}

/// Makes the script at `path` executable.
#[cfg(unix)]
fn executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(EXECUTABLE))
}

/// A file system with no executable bit has nothing to set: the script written is all there is, so this only finds it there.
#[cfg(not(unix))]
fn executable(path: &Path) -> std::io::Result<()> {
    std::fs::metadata(path).map(|_| ())
}

impl HooksPort for HooksGitAdapter {
    fn top(&self, root: &ProductRoot) -> Result<ProductRoot, HooksError> {
        Ok(ProductRoot(
            printed(Command::new(GIT).args(TOP_LEVEL), root)?.0,
        ))
    }

    fn inspect(&self, root: &ProductRoot, name: &HookName) -> Result<Installed, HooksError> {
        let path = path_of(root, name)?;
        if std::fs::symlink_metadata(&path).is_ok_and(|found| found.file_type().is_symlink()) {
            // A symlink, dangling or not, is never asymptote's: it reads as an empty script, which no one owns.
            return Ok(Installed::Script {
                file: HookFile(path.display().to_string().into()),
                text: ScriptText(Box::default()),
            });
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Installed::Script {
                file: HookFile(path.display().to_string().into()),
                text: ScriptText(text.into()),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Installed::Absent),
            Err(error) => Err(HooksError::Unreadable {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    fn install(
        &self,
        root: &ProductRoot,
        name: &HookName,
        script: &ScriptText,
    ) -> Result<(), HooksError> {
        let path = path_of(root, name)?;
        let unwritable = |error: std::io::Error| HooksError::Unwritable {
            path: path.display().to_string(),
            reason: error.to_string(),
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(unwritable)?;
        }
        std::fs::write(&path, script.0.as_bytes()).map_err(unwritable)?;
        executable(&path).map_err(unwritable)
    }

    fn remove(&self, root: &ProductRoot, name: &HookName) -> Result<(), HooksError> {
        let path = path_of(root, name)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(HooksError::Unwritable {
                path: path.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }
}
