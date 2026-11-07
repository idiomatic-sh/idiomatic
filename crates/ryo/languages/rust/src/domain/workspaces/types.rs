//! The types of the `workspaces` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::paths::types::{
    Asked, Entry, EntryName, Matching, Placement, RepoPath, Section,
};
use crate::domain::targets::types::{Manifest, Pattern, Platform};

/// A configuration found above a directory: what it says that the answer depends on, placed in the repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WorkspaceConfiguration {
    /// Where Cargo writes its outputs, when the configuration says: the directory its `target-dir` names, from the directory the
    /// `.cargo` it was found in lies in.
    pub target_dir: Section<Placement>,
}

/// A name tried against a pattern of one segment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SegmentMatch {
    /// The pattern.
    pub pattern: Pattern,
    /// The name.
    pub name: EntryName,
}

/// What the domain has been told of the repository, and what it still wants: the manifests, the directories, the matches of a
/// name with a pattern, the configurations above a directory and the platforms as Cargo shows them. The domain reads nothing: whoever
/// drives a pass reads what `wanted` holds into `seen` and runs the pass again until a pass wants nothing.
#[derive(Clone, Debug, Default)]
pub(crate) struct Looking {
    /// The manifest at a path, or why it cannot be read.
    pub manifests: Asked<RepoPath, Result<Manifest, Reason>>,
    /// What a directory holds.
    pub directories: Asked<RepoPath, Vec<Entry>>,
    /// Whether a name matches a pattern of one segment.
    pub matches: Asked<SegmentMatch, Matching>,
    /// The configurations above a directory of the repository, nearest first.
    pub configurations: Asked<RepoPath, Vec<WorkspaceConfiguration>>,
    /// A platform as Cargo shows it.
    pub platforms: Asked<Platform, PlatformCondition>,
}

/// Why something could not be read: what a reader or a tool said.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Reason(pub Box<str>);

impl std::ops::Deref for Reason {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether the text of a workspace member has a pattern's characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Globbing {
    /// It has: it names several directories.
    Glob,
    /// It has none: it names one.
    Plain,
}

/// Whether a workspace leaves a directory out of its members.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Exclusion {
    /// It does: the directory is excluded, and no member names it.
    Excluded,
    /// It does not.
    Included,
}
