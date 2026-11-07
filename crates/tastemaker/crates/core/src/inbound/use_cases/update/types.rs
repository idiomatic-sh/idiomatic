//! The types of the `update` use case: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::files::types::MachinePath;
use crate::domain::packages::types::{DefinitionName, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::writing::types::Writing;

/// Which entries `update` moves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entries {
    /// Every entry of the root file behind its latest version.
    Every,
    /// The entry of this name.
    Named(DefinitionName),
}

/// The version an entry moves to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionChoice {
    /// The latest its package offers.
    Latest,
    /// This one.
    Version(VersionText),
}

/// What `update` is asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateRequest {
    /// Where to look for the workspace.
    pub start: MachinePath,
    /// Which entries.
    pub entries: Entries,
    /// To which version.
    pub target: VersionChoice,
    /// Whether it writes: `-n` lists what moves, which is what `outdated` was.
    pub writing: Writing,
}

/// One entry moved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Move {
    /// The entry.
    pub entry: DefinitionName,
    /// The version it followed.
    pub from: VersionText,
    /// The version it follows.
    pub to: VersionText,
    /// The changelog sections crossed, when the asset supplies them.
    pub changes: crate::domain::packages::types::ChangeLog,
}

/// What `update` answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateReport {
    /// The file written, or that would be.
    pub file: RepoPath,
    /// The entries moved.
    pub moves: Vec<Move>,
}
