//! The types of the `repository` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::paths::types::RepoPath;

/// What a walk beneath a directory found: every file, and every directory it went into, the one it started at first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    /// The files, sorted.
    pub files: Vec<RepoPath>,
    /// The directories, sorted: a file added or removed beneath one changes its stamp.
    pub directories: Vec<RepoPath>,
    /// The symbolic links, sorted: paths of the tree, never followed and never read.
    pub links: Vec<RepoPath>,
}

/// Whether a path exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    /// It does.
    Present,
    /// It does not.
    Absent,
}

/// Paths of the repository, sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RepoPaths(pub Vec<RepoPath>);

impl std::ops::Deref for RepoPaths {
    type Target = [RepoPath];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for RepoPaths {
    type Item = RepoPath;
    type IntoIter = std::vec::IntoIter<RepoPath>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RepoPaths {
    type Item = &'a RepoPath;
    type IntoIter = std::slice::Iter<'a, RepoPath>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// The name of what marks a directory, looked for at and above a start: a workspace's `tastemaker.toml`, a repository's `.git`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marker<'a>(pub &'a str);
