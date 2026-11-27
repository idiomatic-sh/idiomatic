//! The types of the `applying` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::{BTreeMap, BTreeSet};

use tastemaker_values::domain::applications::types::ParamSet;
use tastemaker_values::domain::packages::types::Release;
use tastemaker_values::domain::paths::types::RepoPath;

/// A release with one set of params: what reaches files.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Application {
    /// The release.
    pub release: Release,
    /// Its params.
    pub params: ParamSet,
}

/// The files one application reaches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Reached(pub BTreeSet<RepoPath>);

/// Each application, and the files it reaches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Applied(pub BTreeMap<Application, Reached>);

/// The files one application reaches: one rule of the lint call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    /// The release.
    pub release: Release,
    /// Its params.
    pub params: ParamSet,
    /// The files it reaches.
    pub files: BTreeSet<RepoPath>,
}

/// A file one release reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ReachedFile<'a> {
    /// The release.
    pub release: &'a Release,
    /// The file.
    pub file: &'a RepoPath,
}
