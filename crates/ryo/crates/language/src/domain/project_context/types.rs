//! Plain project search authority and candidate observation data.
use crate::domain::files::types::FilePath;
use std::sync::Arc;

/// The host's authority for resolving names within a project. An empty declared list proves no local root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectSearchScope {
    /// The host did not declare its import search roots.
    #[default]
    Undeclared,
    /// Repository-relative roots, in search order, including the empty repository root when declared.
    Declared(Vec<FilePath>),
}

/// Whether a Given inventory can prove that an unlisted entry is absent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ProjectInventoryCompleteness {
    /// An unlisted candidate remains unknown.
    #[default]
    Partial,
    /// All entries relevant within the declared search scope are listed.
    CompleteWithinDeclaredScope,
}

/// Host-provided entry observations for a Given project; no disk observation is permitted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GivenProjectInventory {
    /// Exact entry states supplied by the host.
    pub entries: Vec<EntryObservation>,
    /// The host's completeness claim for unlisted entries.
    pub completeness: ProjectInventoryCompleteness,
}

/// An entry observed at a repository-relative path, without reading its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryObservation {
    /// Candidate path requested by the project.
    pub path: FilePath,
    /// The observed kind or uncertainty.
    pub state: EntryState,
}

/// One immutable engine-owned batch of entry observations, sorted by exact path.
/// Duplicate paths remain visible so a malformed batch cannot certify absence.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ObservedEntries(pub(crate) Arc<[EntryObservation]>);

/// The distinct outcomes of an entry observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryState {
    /// A regular source file.
    File,
    /// A directory, including a namespace package without an initializer.
    Directory,
    /// An entry of another kind.
    Other,
    /// The entry is absent within the observed scope.
    Missing,
    /// An observation failed or the host has not declared enough inventory.
    Unknown,
}
