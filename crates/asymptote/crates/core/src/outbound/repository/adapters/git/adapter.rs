//! The repository port, implemented with the git command line: one engine for every question, so the renames it
//! carries records through are the ones git itself reports, and nothing else of git is linked.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

use super::constants::{
    AT_PATH, CHECK_IGNORE, DIFF, END, HEAD, IGNORED, INDEX, INDEX_FILE, INTENT_TO_ADD, LITERAL,
    LOG_ADDED, LOG_RENAMES, LS_FILES, LS_TREE, MERGE_BASE, NONE_IGNORED, PATHS, PEEL, RANGE,
    REV_PARSE, SEPARATOR, SHOW, UNTRACKED,
};
use super::schemas::{history, ignored, introductions, is_object_id, paths, renames};
use super::settings::RepositoryGitSettings;
use crate::domain::git::constants::{GIT, HOOK_VARIABLES, NOT_A_REPOSITORY, TOP_LEVEL};
use crate::domain::inputs::service::beneath;
use crate::domain::manifest::types::{Pathspec, ProductRoot};
use crate::domain::operations::types::{OperationText, Revision};
use crate::domain::renames::types::{Head, HistoryStart};
use crate::domain::violations::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::{
    Answers, Files, GitArguments, IgnoredInputs, Introductions, Printed, RenameHistory, Renames,
    TrackedFile, TrackedFiles, Worktree,
};

/// The git repository adapter.
#[derive(Debug)]
pub(crate) struct RepositoryGitAdapter {
    /// The working tree the edge listed.
    pub settings: RepositoryGitSettings,
}

impl RepositoryGitAdapter {
    /// The adapter over the product at `root` and `worktree`, the working tree the edge listed once.
    pub(crate) const fn new(
        root: ProductRoot,
        worktree: Result<Worktree, RepositoryError>,
    ) -> Self {
        Self {
            settings: RepositoryGitSettings { root, worktree },
        }
    }
}

/// Every file of the working tree of the repository `start` is in, listed once from its top: two git commands for
/// every product in it.
fn listed(start: &ProductRoot) -> Result<Worktree, RepositoryError> {
    let top = git(start, &GitArguments(TOP_LEVEL.to_vec()))?;
    let top = std::path::PathBuf::from(top.0.trim());
    let at = ProductRoot(top.display().to_string().into());
    let mut files = paths(&git(&at, &GitArguments(LS_FILES.to_vec()))?.0);
    files.sort();
    files.dedup();
    Ok(Worktree { top, files })
}

/// The path of the product at `root` from the repository's top `top`.
fn from_top(root: &ProductRoot, top: &Path) -> Result<RepoPath, RepositoryError> {
    let canonical =
        Path::new(&*root.0)
            .canonicalize()
            .map_err(|error| RepositoryError::Failed {
                reason: error.to_string(),
            })?;
    // Both sides canonical: git prints its top in its own form (on Windows `C:/Users/...`), never a short name or a
    // link, which the product's root may be written in.
    let top = top
        .canonicalize()
        .map_err(|error| RepositoryError::Failed {
            reason: error.to_string(),
        })?;
    let beneath = canonical
        .strip_prefix(&top)
        .map_err(|error| RepositoryError::Failed {
            reason: format!("`{}` is not beneath `{}`: {error}", root.0, top.display()),
        })?;
    Ok(RepoPath(
        beneath
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(SEPARATOR)
            .into(),
    ))
}

/// What `git <arguments>` printed in the product's directory, or why it failed.
fn git(root: &ProductRoot, arguments: &GitArguments<'_>) -> Result<Printed, RepositoryError> {
    git_over(root, arguments, None)
}

/// What `git <arguments>` printed over the index file `index`, the repository's own when none, every pathspec read
/// literally over another.
fn git_over(
    root: &ProductRoot,
    arguments: &GitArguments<'_>,
    index: Option<&Path>,
) -> Result<Printed, RepositoryError> {
    answered(root, arguments, index, Answers::OnSuccess)
}

/// Runs `git <arguments>` over the index file `index` for its effect alone: nothing it prints is wanted.
fn applied(
    root: &ProductRoot,
    arguments: &GitArguments<'_>,
    index: &Path,
) -> Result<(), RepositoryError> {
    // The run's output is std's record of it; only that it succeeded counts.
    drop(completed(root, arguments, Some(index), Answers::OnSuccess)?);
    Ok(())
}

/// What `git <arguments>` printed over the index file `index`, when its status is one that `answers`.
fn answered(
    root: &ProductRoot,
    arguments: &GitArguments<'_>,
    index: Option<&Path>,
    answers: Answers,
) -> Result<Printed, RepositoryError> {
    let output = completed(root, arguments, index, answers)?;
    Ok(Printed(String::from_utf8_lossy(&output.stdout).into()))
}

/// The finished run of `git <arguments>` over the index file `index`, when its status is one that `answers`.
fn completed(
    root: &ProductRoot,
    arguments: &GitArguments<'_>,
    index: Option<&Path>,
    answers: Answers,
) -> Result<std::process::Output, RepositoryError> {
    // A hook's variables are removed before the index file is set, which is one of them.
    let output = HOOK_VARIABLES
        .iter()
        .fold(
            Command::new(GIT)
                .args(index.map(|_| LITERAL))
                .args(&arguments.0)
                .current_dir(Path::new(&*root.0))
                .stdin(Stdio::null()),
            |command, variable| command.env_remove(variable),
        )
        .envs(index.map(|index| (INDEX_FILE, index)))
        .output()
        .map_err(|error| RepositoryError::Failed {
            reason: error.to_string(),
        })?;
    if output.status.success()
        || (answers == Answers::OnSuccessOrNoneIgnored
            && output.status.code() == Some(NONE_IGNORED))
    {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains(NOT_A_REPOSITORY) {
        return Err(RepositoryError::NotARepository {
            location: root.0.to_string(),
        });
    }
    Err(RepositoryError::Failed {
        reason: stderr.trim().to_owned(),
    })
}

/// The commit `revision` names, when it names one.
fn commit(root: &ProductRoot, revision: &Revision) -> Result<Option<Revision>, RepositoryError> {
    let peeled = format!("{}{PEEL}", revision.0);
    match git(
        root,
        &GitArguments(REV_PARSE.iter().copied().chain([&*peeled]).collect()),
    ) {
        Ok(id) if is_object_id(id.0.trim()) => Ok(Some(Revision(id.0.trim().into()))),
        Ok(_) | Err(RepositoryError::Failed { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The commit `revision` names, refused when it names none.
fn known(root: &ProductRoot, revision: &Revision) -> Result<Revision, RepositoryError> {
    commit(root, revision)?.ok_or_else(|| RepositoryError::UnknownRevision {
        revision: revision.0.to_string(),
    })
}

impl RepositoryPort for RepositoryGitAdapter {
    fn worktree(&self) -> Result<Worktree, RepositoryError> {
        listed(&self.settings.root)
    }

    fn head(&self) -> Result<Head, RepositoryError> {
        let root = &self.settings.root;
        match commit(root, &Revision(HEAD.into())) {
            Ok(Some(revision)) => Ok(Head::Commit(revision)),
            Ok(None) | Err(RepositoryError::NotARepository { .. }) => Ok(Head::Detached),
            Err(error) => Err(error),
        }
    }

    fn commit(&self, revision: &Revision) -> Result<Revision, RepositoryError> {
        let root = &self.settings.root;
        known(root, revision)
    }

    fn merge_base(&self, one: &Revision, other: &Revision) -> Result<Revision, RepositoryError> {
        let root = &self.settings.root;
        let one = known(root, one)?;
        let other = known(root, other)?;
        match git(root, &GitArguments(vec![MERGE_BASE, &one.0, &other.0])) {
            Ok(base) if is_object_id(base.0.trim()) => Ok(Revision(base.0.trim().into())),
            Ok(_) | Err(RepositoryError::Failed { .. }) => Err(RepositoryError::UnknownRevision {
                revision: other.0.to_string(),
            }),
            Err(error) => Err(error),
        }
    }

    fn renames_to_worktree(&self, from: &Revision) -> Result<Renames, RepositoryError> {
        let root = &self.settings.root;
        // Moving a file with `mv` is a rename before `git add` says so. Git's diff reads only the paths its index
        // holds, so the untracked files are added, intent-to-add, to a copy of the index the diff then reads; the
        // repository's own index is never written.
        let arguments = GitArguments(DIFF.iter().copied().chain([&*from.0]).collect());
        if git(root, &GitArguments(UNTRACKED.to_vec()))?.0.is_empty() {
            return Ok(Renames(renames(&git(root, &arguments)?.0)));
        }
        let failed = |error: std::io::Error| RepositoryError::Failed {
            reason: error.to_string(),
        };
        let index = git(root, &GitArguments(INDEX.to_vec()))?;
        let index = Path::new(index.0.trim_end_matches('\n'));
        let copy = tempfile::NamedTempFile::new().map_err(failed)?;
        if index.is_file() {
            let _: u64 = std::fs::copy(index, copy.path()).map_err(failed)?;
        }
        applied(root, &GitArguments(INTENT_TO_ADD.to_vec()), copy.path())?;
        Ok(Renames(renames(
            &git_over(root, &arguments, Some(copy.path()))?.0,
        )))
    }

    fn rename_history(
        &self,
        from: &HistoryStart,
        to: &Revision,
    ) -> Result<RenameHistory, RepositoryError> {
        let root = &self.settings.root;
        let range = match from {
            HistoryStart::Beginning => to.0.to_string(),
            HistoryStart::After(revision) => format!("{}{RANGE}{}", known(root, revision)?.0, to.0),
        };
        history(
            &git(
                root,
                &GitArguments(LOG_RENAMES.iter().copied().chain([&*range]).collect()),
            )?
            .0,
        )
        .map(RenameHistory)
    }

    fn introductions(
        &self,
        to: &Revision,
        prefix: &Pathspec,
    ) -> Result<Introductions, RepositoryError> {
        let root = &self.settings.root;
        introductions(
            &git(
                root,
                &GitArguments(
                    LOG_ADDED
                        .iter()
                        .copied()
                        .chain([END, &*to.0, PATHS, &*prefix.0])
                        .collect(),
                ),
            )?
            .0,
        )
        .map(Introductions)
    }

    fn read_all_at(
        &self,
        revision: &Revision,
        prefix: &Pathspec,
    ) -> Result<TrackedFiles, RepositoryError> {
        let root = &self.settings.root;
        let revision = known(root, revision)?;
        let listed = git(
            root,
            &GitArguments(
                LS_TREE
                    .iter()
                    .copied()
                    .chain([&*revision.0, PATHS, &*prefix.0])
                    .collect(),
            ),
        )?;
        let mut files = Vec::new();
        for path in paths(&listed.0) {
            let at = format!("{}{AT_PATH}{}", revision.0, path.0);
            let text = git(root, &GitArguments(vec![SHOW, &at]))?;
            files.push(TrackedFile {
                path,
                text: OperationText(text.0),
            });
        }
        Ok(TrackedFiles(files))
    }

    fn files(&self, inputs: &[Pathspec]) -> Result<Files, RepositoryError> {
        let root = &self.settings.root;
        let worktree = self.settings.worktree.as_ref().map_err(Clone::clone)?;
        let base = from_top(root, &worktree.top)?;
        let mut files: Vec<RepoPath> = inputs
            .iter()
            .flat_map(|input| beneath(&worktree.files, &base, input))
            .collect();
        files.sort();
        files.dedup();
        Ok(Files(files))
    }

    fn ignored(&self, inputs: &[Pathspec]) -> Result<IgnoredInputs, RepositoryError> {
        let root = &self.settings.root;
        if inputs.is_empty() {
            return Ok(IgnoredInputs::default());
        }
        let asked = CHECK_IGNORE
            .iter()
            .copied()
            .chain(inputs.iter().map(|input| &*input.0));
        let answered = answered(
            root,
            &GitArguments(asked.collect()),
            None,
            Answers::OnSuccessOrNoneIgnored,
        )?;
        let found = ignored(&answered.0)
            .into_iter()
            .filter_map(|path| {
                inputs
                    .iter()
                    .find(|input| &*input.0 == path)
                    .map(|input| (input, path))
            })
            .map(|(input, path)| {
                let arguments = IGNORED.iter().copied().chain([path]);
                let mut files = paths(&git(root, &GitArguments(arguments.collect()))?.0);
                files.sort();
                Ok((input.clone(), files))
            })
            .collect::<Result<BTreeMap<_, _>, RepositoryError>>()?;
        Ok(IgnoredInputs(found))
    }
}
