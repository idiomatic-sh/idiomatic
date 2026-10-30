//! The memory adapter's own plain types: the trees it holds, the scratch directory it hands out, and what a walk of one
//! keeps.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, PoisonError};

use crate::domain::files::types::{DirectoryName, FileText, MachinePath, SkippedDirectory};
use crate::domain::paths::types::RepoPath;

/// The trees an adapter holds, by the scratch directory each is, each behind a lock of its own: the scenarios checked at
/// once never wait on one another.
#[derive(Debug, Default)]
pub(crate) struct Trees(pub Mutex<HashMap<MachinePath, Arc<Mutex<Tree>>>>);

/// A directory of the trees, bound to its root: the tree it is in looked up at each call, so a project dropped leaves it
/// holding nothing, as a directory removed from a disk.
#[derive(Debug)]
pub(crate) struct RepositoryMemoryRepository {
    /// Its root.
    pub root: MachinePath,
    /// The directories a walk never goes into, at any depth, from the linked languages.
    pub skipped: Vec<SkippedDirectory>,
    /// The store's directory, never walked.
    pub store: DirectoryName,
    /// The adapter's trees.
    pub trees: Arc<Trees>,
    /// Where the nearest `tastemaker.toml` is, when the directory was found by it; its root otherwise.
    pub workspace: RepoPath,
    /// Where the search for it started, when it was found; its root otherwise.
    pub start: RepoPath,
}

/// A scratch directory held in memory, bound to its root: its tree is forgotten when it is dropped.
#[derive(Debug)]
pub(crate) struct RepositoryMemoryProject {
    /// The scratch directory, bound.
    pub repository: RepositoryMemoryRepository,
}

impl Drop for RepositoryMemoryProject {
    fn drop(&mut self) {
        drop(
            self.repository
                .trees
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&self.repository.root),
        );
    }
}

/// One tree: each file a scratch directory holds, by its path.
#[derive(Debug, Default)]
pub(crate) struct Tree {
    /// The files, by their path below the scratch directory.
    pub files: BTreeMap<RepoPath, HeldFile>,
}

/// A file the tree holds: its text, and how many times it was written.
#[derive(Debug)]
pub(crate) struct HeldFile {
    /// Its text.
    pub text: FileText,
    /// How many times it was written, which its stamp carries.
    pub writes: Writes,
}

/// How many times a file was written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Writes(pub u64);

/// Whether a path is a file or a directory, as an ignore file matches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A file.
    File,
    /// A directory.
    Directory,
}

/// Whether the ignore files above a path leave it out of a walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ignoring {
    /// They leave it out.
    Ignored,
    /// They keep it.
    Kept,
}

/// Whether a walk goes into a directory it comes upon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entering {
    /// It goes in.
    Enters,
    /// It never does: `.git`, the store, a directory a language skips, or one its ignore files leave out.
    Skips,
}

/// The name of an ignore file, as `.gitignore`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct IgnoreFile(pub &'static str);

/// The ignore files a walk compiled, each once, by its path: none for one the tree does not hold or that does not compile.
#[derive(Debug, Default)]
pub(crate) struct Matchers(pub HashMap<RepoPath, Option<ignore::gitignore::Gitignore>>);

/// Where a path of the machine is among the scratch directories: the one it is or is within, and the path below it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ScratchPlace {
    /// The scratch directory.
    pub scratch: MachinePath,
    /// The path below it; the empty path for the directory itself.
    pub below: RepoPath,
    /// The path asked of it, beneath `below`.
    pub target: RepoPath,
}
