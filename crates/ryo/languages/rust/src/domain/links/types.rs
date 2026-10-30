//! The types of the `links` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeSet;

use ryo::languages::{FilePath, Offset};

use crate::domain::module_tree::types::{Facts, Spelling, Unfollowed};
use crate::domain::package_index::types::ListIndex;
use crate::domain::packages::types::Refusal;
use crate::domain::paths::types::RepoPath;

/// What examining one file came to: the dependencies it has, and its place when it was analyzed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Examination {
    /// The dependencies of the file.
    pub dependencies: Vec<Link>,
    /// The file's place, when all its written paths were examined; nothing when they were not.
    pub analyzed: Vec<ListIndex>,
}

/// A path written in one file that names what another owns, by the files' places among the repository's.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Link {
    /// The file the path is written in.
    pub source: ListIndex,
    /// The file that owns what it names.
    pub target: ListIndex,
    /// Where the path starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The path as the file writes it, its whitespace left out.
    pub written: Spelling,
}

/// What linking the files of a repository came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Linked {
    /// The dependencies, by source file, then target, then place.
    pub dependencies: Vec<Link>,
    /// The files whose written paths were all examined, in order.
    pub analyzed: Vec<ListIndex>,
    /// The modules that could not be followed, and why: dependencies through them are not found.
    pub gaps: Vec<Unfollowed>,
    /// The file each of `gaps` is about, in the same order: `u32::MAX` when the file that declares the module is not listed.
    pub gap_files: Vec<ryo::languages::FileId>,
    /// What the manifests declared, when the project has a manifest of the language: read once when the project was resolved.
    pub packages: Option<crate::domain::packages::types::Packages>,
}

/// A file and the package it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Placed {
    /// The file's place among the repository's.
    pub place: ListIndex,
    /// The package's manifest, relative to the repository root.
    pub package: RepoPath,
}

/// What the manifests say about which package holds each file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Packaged {
    /// The files that belong to a package, in the order of their places.
    pub files: Vec<Placed>,
    /// The manifests that could not be read: the files they would have held are missing.
    pub refused: Vec<Refusal>,
    /// The number of the file of each manifest the packages name: its place among the files the run lists, or after them.
    pub listed: std::collections::BTreeMap<RepoPath, ryo::languages::FileId>,
}

/// One dependency of a file on another, before the place of the target is known: the target and where the path is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UnplacedLink<'a> {
    /// The file the path names.
    pub target: &'a RepoPath,
    /// Where the path starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The path as the file writes it, its whitespace left out.
    pub written: &'a Spelling,
}

/// The module trees drawn for the crate roots of one package.
#[derive(Clone, Debug)]
pub(crate) struct Drawn {
    /// The package's directory.
    pub directory: RepoPath,
    /// One tree for each crate root of the package.
    pub trees: Vec<crate::domain::module_tree::types::ModuleTree>,
}

/// What every step of linking one run shares: the files it kept, the files it lists in the order of their places, and the facts of each
/// source file, by its place.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Run<'a> {
    /// The files whose paths are not outputs of the build.
    pub kept: &'a BTreeSet<RepoPath>,
    /// The files of the run, in the order of their places and in the order of their paths.
    pub files: &'a [FilePath],
    /// The facts of each source file, by its place.
    pub facts: &'a [Option<Facts>],
}
