//! The types of the `explain` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::explanations::types::Section;
use crate::domain::files::types::MachinePath;
use crate::domain::paths::types::RepoPath;

/// What `explain` is asked about: a path on disk, else `<kind>/<name>`, else a topic; nothing for the whole workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// The whole workspace: its scopes and what they place.
    Workspace,
    /// This path, a definition or a topic, resolved in that order.
    Named(NameAsked),
    /// Every topic and definition, written to this directory.
    Everything(MachinePath),
}

/// What `explain` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplainRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// What about.
    pub asked: Asked,
}

/// What `explain` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplainReport {
    /// The explanation.
    pub sections: Vec<Section>,
    /// The files written, for `--all`.
    pub written: Vec<RepoPath>,
}

/// What the caller names: a path, `<kind>/<name>` or a topic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameAsked(pub Box<str>);

/// The workspace nearest the start, read, and where it was found.
#[derive(Debug)]
pub struct Opened {
    /// Where the workspace was found.
    pub found: std::sync::Arc<dyn crate::outbound::repository::interface::FoundRepository>,
    /// The workspace, read.
    pub workspace: crate::domain::definitions::types::LoadedWorkspace,
}
