//! The types of the `linting` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::definitions::types::LocalPackage;
use crate::domain::paths::types::RepoPath;

/// The packages a workspace holds among its own files, and the files it governs, theirs left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Partitioned {
    /// The packages that lie among the workspace's files.
    pub local_packages: Vec<LocalPackage>,
    /// The files the workspace governs.
    pub governed: Vec<RepoPath>,
}

/// Why this ryo refuses a query, as it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusalReason(pub Box<str>);

/// The paths of a listing one to a line, as bytes: what the digest of a plan's inputs reads of the listing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListingText(pub Vec<u8>);
