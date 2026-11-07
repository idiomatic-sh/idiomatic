//! The types of the `repository` port: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeMap;

use crate::domain::manifest::types::Pathspec;
use crate::domain::operations::types::OperationText;
use crate::domain::renames::types::{Introduction, Rename, RenameEvent};
use crate::domain::violations::types::RepoPath;

/// A file as a revision holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedFile {
    /// Its path, relative to the product.
    pub path: RepoPath,
    /// Its text.
    pub text: OperationText,
}

/// The renames git reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Renames(pub Vec<Rename>);

/// First-parent commits with their renames, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenameHistory(pub Vec<RenameEvent>);

/// The commits that added files of the lock, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Introductions(pub Vec<Introduction>);

/// The files of a revision beneath a path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackedFiles(pub Vec<TrackedFile>);

/// Every file of a repository's working tree, tracked and untracked but not ignored, listed once for every product
/// in it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Worktree {
    /// The repository's top, the directory its paths are written from.
    pub top: std::path::PathBuf,
    /// The files, each a path from the top, sorted.
    pub files: Vec<RepoPath>,
}

/// Files of the working tree, sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Files(pub Vec<RepoPath>);

/// For each input git ignores itself, the files beneath it, which git ignores too.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IgnoredInputs(pub BTreeMap<Pathspec, Vec<RepoPath>>);

/// What one git command printed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Printed(pub Box<str>);

/// Which of git's exit statuses answer a question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answers {
    /// Success alone.
    OnSuccess,
    /// Success, or the status `git check-ignore` exits with when no path is ignored.
    OnSuccessOrNoneIgnored,
}

/// A git command line, after `git`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitArguments<'a>(pub Vec<&'a str>);
