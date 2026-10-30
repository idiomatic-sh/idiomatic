//! The fake of the `repository` port: each answer given up front.

use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::{Head, HistoryStart, Introduction, RenameEvent};
use crate::domain::violations::types::RepoPath;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::{
    Files, IgnoredInputs, Introductions, RenameHistory, Renames, TrackedFile, TrackedFiles,
    Worktree,
};

/// What the core's tests stand in for `repository` with.
#[derive(Debug)]
pub struct RepositoryFake {
    /// The head.
    pub head: Head,
    /// The merge base of any two revisions.
    pub base: Revision,
    /// The renames to the working tree.
    pub worktree: Result<Renames, RepositoryError>,
    /// The rename history.
    pub history: Vec<RenameEvent>,
    /// The introductions.
    pub introduced: Vec<Introduction>,
    /// The lock's files at the merge base.
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - a field of the fake a scene sets, beside the others it sets from other files; the fake keeps one visibility for every field (clippy::partial_pub_fields)
    pub at_base: Vec<TrackedFile>,
    /// The lock's files at any other revision.
    pub at_other: Vec<TrackedFile>,
    /// The files of the product.
    pub files: Vec<RepoPath>,
    /// How many times git's rename detection was asked: the renames to the working tree, or the rename history.
    pub renamed: std::sync::atomic::AtomicUsize,
}

impl Default for RepositoryFake {
    fn default() -> Self {
        Self {
            head: Head::Commit(Revision("head".into())),
            base: Revision("base".into()),
            worktree: Ok(Renames(Vec::new())),
            history: Vec::new(),
            introduced: Vec::new(),
            at_base: Vec::new(),
            at_other: Vec::new(),
            files: Vec::new(),
            renamed: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl RepositoryPort for RepositoryFake {
    fn worktree(&self) -> Result<Worktree, RepositoryError> {
        Ok(Worktree {
            top: std::path::PathBuf::new(),
            files: self.files.clone(),
        })
    }

    fn head(&self) -> Result<Head, RepositoryError> {
        Ok(self.head.clone())
    }

    fn commit(&self, revision: &Revision) -> Result<Revision, RepositoryError> {
        if revision.0.starts_with("no-such") {
            return Err(RepositoryError::UnknownRevision {
                revision: revision.0.to_string(),
            });
        }
        match (&*revision.0, &self.head) {
            ("HEAD", Head::Commit(head)) => Ok(head.clone()),
            ("HEAD", Head::Detached) => Err(RepositoryError::UnknownRevision {
                revision: revision.0.to_string(),
            }),
            (name, _) => Ok(Revision(["commit-of-", name].concat().into())),
        }
    }

    fn merge_base(&self, one: &Revision, other: &Revision) -> Result<Revision, RepositoryError> {
        let known = |revision: &Revision| !revision.0.starts_with("no-such");
        if !known(one) || !known(other) {
            return Err(RepositoryError::UnknownRevision {
                revision: other.0.to_string(),
            });
        }
        Ok(if one == other {
            one.clone()
        } else {
            self.base.clone()
        })
    }

    fn renames_to_worktree(&self, _from: &Revision) -> Result<Renames, RepositoryError> {
        let _asked_before = self
            .renamed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.worktree.clone()
    }

    fn rename_history(
        &self,
        _from: &HistoryStart,
        _to: &Revision,
    ) -> Result<RenameHistory, RepositoryError> {
        let _asked_before = self
            .renamed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(RenameHistory(self.history.clone()))
    }

    fn introductions(
        &self,
        _to: &Revision,
        _prefix: &Pathspec,
    ) -> Result<Introductions, RepositoryError> {
        Ok(Introductions(self.introduced.clone()))
    }

    fn read_all_at(
        &self,
        revision: &Revision,
        _prefix: &Pathspec,
    ) -> Result<TrackedFiles, RepositoryError> {
        Ok(TrackedFiles(if revision == &self.base {
            self.at_base.clone()
        } else {
            self.at_other.clone()
        }))
    }

    fn files(&self, inputs: &[Pathspec]) -> Result<Files, RepositoryError> {
        // A pathspec names the root, a file, a directory and every file beneath it, or, starting with `*`, every file
        // ending as the rest of it does.
        let names = |spec: &Pathspec, file: &RepoPath| {
            let spec = spec.0.trim_end_matches('/');
            spec.strip_prefix('*').map_or_else(
                || spec == "." || *file.0 == *spec || file.0.starts_with(&format!("{spec}/")),
                |ending| file.0.ends_with(ending),
            )
        };
        Ok(Files(
            self.files
                .iter()
                .filter(|file| inputs.iter().any(|spec| names(spec, file)))
                .cloned()
                .collect(),
        ))
    }

    fn ignored(&self, _inputs: &[Pathspec]) -> Result<IgnoredInputs, RepositoryError> {
        Ok(IgnoredInputs::default())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
