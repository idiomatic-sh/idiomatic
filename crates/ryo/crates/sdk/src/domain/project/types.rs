//! The SDK's `project` concept: the project lookups a run is given and answers: the search scope, the given inventory and the dependencies examined.

use crate::domain::file::types::RelativePath;

/// Explicit host authority for project import roots; an empty declared list is meaningful.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectSearchScope {
    /// No roots were declared, so local membership cannot be proved absent.
    #[default]
    Undeclared,
    /// Repository-relative search roots, in the order the host uses them.
    Declared(Vec<RelativePath>),
}

/// How much of a Given project's search scope the host inventoried.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectInventoryCompleteness {
    /// An entry omitted from the inventory remains unknown.
    #[default]
    Partial,
    /// Omitted entries within the declared scope are absent.
    CompleteWithinDeclaredScope,
}

/// One host-observed candidate entry, separate from source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectEntryObservation {
    /// Repository-relative candidate path.
    pub path: RelativePath,
    /// Its observed kind.
    pub state: ProjectEntryState,
}

/// Entry kinds available to a Given project's namespace lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectEntryState {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

/// Explicit Given inventory and its absence authority; no disk probing is implied.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GivenProjectInventory {
    pub entries: Vec<ProjectEntryObservation>,
    pub completeness: ProjectInventoryCompleteness,
}

/// The bytes of a project dependency, borrowed, which `DependencyFingerprint::from` fingerprints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DependencyBytes<'a>(pub &'a [u8]);

/// An opaque project dependency content fingerprint. Made from `DependencyBytes` to compare bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyFingerprint(pub [u8; 16]);

/// One file the project examined, whether present or absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDependency {
    /// Path from the run root, possibly containing `../`.
    pub path: RelativePath,
    /// The observed bytes' fingerprint, or an explicitly missing file.
    pub state: ProjectDependencyState,
}

/// State of a required project lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectDependencyState {
    /// Content fingerprint supplied by the engine's digest adapter.
    Present(DependencyFingerprint),
    /// The path was examined and no file was present.
    Missing,
}
