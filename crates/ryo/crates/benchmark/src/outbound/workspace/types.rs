//! Values carried by the reproducible workspace boundary.
/// The immutable commit recorded when this workspace was prepared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitRevision(pub String);
/// The exact tracked diff retained for checked restoration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppliedPatch(pub Vec<u8>);
/// Sorted repository-relative tracked source paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedFiles(pub Vec<String>);
