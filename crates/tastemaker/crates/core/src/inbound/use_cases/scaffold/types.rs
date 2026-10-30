//! The types of the `scaffold` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use crate::domain::entries::types::Setting;
use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;

/// What `scaffold` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// The entry, by its alias or its definition's name, or as `explain` names it.
    pub entry: DefinitionName,
    /// The directory a spec entry's instance is written at, relative to the start.
    pub at: RepoPath,
    /// The captures and params given, by name.
    pub set: BTreeMap<Setting, Setting>,
    /// Whether it writes.
    pub writing: Writing,
}

/// What `scaffold` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldReport {
    /// The files written, or that would be.
    pub written: Vec<RepoPath>,
    /// The files that already exist, which it never overwrites.
    pub kept: Vec<RepoPath>,
}

/// The workspace found, and everything it names read.
#[derive(Debug)]
pub struct Workspace {
    /// Where the workspace was found.
    pub found: std::sync::Arc<dyn crate::outbound::repository::interface::FoundRepository>,
    /// The workspace and everything it names, read.
    pub loaded: crate::domain::definitions::types::LoadedWorkspace,
}
