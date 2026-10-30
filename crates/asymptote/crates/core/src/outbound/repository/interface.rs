//! What the `repository` port asks of the outside, in the core's own vocabulary. Every path is relative to the
//! product's root.

use super::errors::RepositoryError;
use super::types::{
    Files, IgnoredInputs, Introductions, RenameHistory, Renames, TrackedFiles, Worktree,
};
use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::Revision;
use crate::domain::renames::types::{Head, HistoryStart};

/// What every `repository` adapter implements. Each question fails with `NotARepository` where there is none, and
/// with `Failed` where git cannot answer.
pub trait RepositoryPort: std::fmt::Debug + Send + Sync {
    /// Every file of the working tree of the repository the product is in, tracked and untracked but not ignored, listed
    /// once from its top: the edge lists it once and gives it to each product's adapter, whose `files` read it.
    ///
    /// # Errors
    ///
    /// `NotARepository` when the product is in none, `Failed` when git fails.
    fn worktree(&self) -> Result<Worktree, RepositoryError>;

    /// The commit the working tree stands on; `Detached` with no repository or no commit.
    ///
    /// # Errors
    ///
    /// `Failed` when git cannot answer.
    fn head(&self) -> Result<Head, RepositoryError>;

    /// The commit `revision` names, as its full object id: a branch, a tag, `HEAD` or an abbreviated id.
    ///
    /// # Errors
    ///
    /// `UnknownRevision` when `revision` names no commit this clone holds.
    fn commit(&self, revision: &Revision) -> Result<Revision, RepositoryError>;

    /// The common ancestor of two revisions.
    ///
    /// # Errors
    ///
    /// `UnknownRevision` when either names no commit or they share no ancestor.
    fn merge_base(&self, one: &Revision, other: &Revision) -> Result<Revision, RepositoryError>;

    /// The renames between `from` and the index and working tree.
    ///
    /// # Errors
    ///
    /// The errors every question has.
    fn renames_to_worktree(&self, from: &Revision) -> Result<Renames, RepositoryError>;

    /// Every first-parent commit after `from` up to `to`, with its renames, oldest first.
    ///
    /// # Errors
    ///
    /// `UnknownRevision` when `from` names no commit this clone holds.
    fn rename_history(
        &self,
        from: &HistoryStart,
        to: &Revision,
    ) -> Result<RenameHistory, RepositoryError>;

    /// The first-parent commit that added each file beneath `prefix`, up to `to`, oldest first.
    ///
    /// # Errors
    ///
    /// The errors every question has.
    fn introductions(
        &self,
        to: &Revision,
        prefix: &Pathspec,
    ) -> Result<Introductions, RepositoryError>;

    /// Every file beneath `prefix` as `revision` holds it.
    ///
    /// # Errors
    ///
    /// `UnknownRevision` when `revision` names no commit.
    fn read_all_at(
        &self,
        revision: &Revision,
        prefix: &Pathspec,
    ) -> Result<TrackedFiles, RepositoryError>;

    /// The files `inputs` name, tracked or untracked but not ignored, sorted.
    ///
    /// # Errors
    ///
    /// The errors every question has.
    fn files(&self, inputs: &[Pathspec]) -> Result<Files, RepositoryError>;

    /// The inputs git ignores, each with the files beneath it: an input named by its own path is read though git
    /// ignores it. One question for every input.
    ///
    /// # Errors
    ///
    /// The errors every question has.
    fn ignored(&self, inputs: &[Pathspec]) -> Result<IgnoredInputs, RepositoryError>;
}
