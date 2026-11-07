//! The types of the `package_index` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::paths::types::RepoPath;

/// The packages of the repository: for each, its directory and the root files of its crates, and the directories each reaches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Index {
    /// Each package by the directory of its manifest, with the crate roots among the files.
    pub directories: Vec<Package>,
    /// The directories each package holds, by its place: its own, and the directory of every target it declares outside it, and whether it is its own.
    pub extents: Vec<PackageDirectory>,
}

/// A package: the directory of its manifest and the files its crates are rooted in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Package {
    /// The directory of the package's manifest.
    pub directory: RepoPath,
    /// The crate roots among the files.
    pub roots: Vec<RepoPath>,
}

/// A directory a package holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PackageDirectory {
    /// The directory.
    pub directory: RepoPath,
    /// The package that holds it, by its place among the packages.
    pub package: ListIndex,
    /// Whether it is the package's own directory, not the directory of a target it declares outside.
    pub reach: Reach,
}

/// The place of a file among the repository's, or of a package among the repository's packages: the number it has in a list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ListIndex(pub usize);

/// Whether an extent is the directory of a package itself or the directory of a target it declares elsewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Reach {
    /// The directory of a target outside the package's own.
    Target,
    /// The package's own directory.
    Own,
}
