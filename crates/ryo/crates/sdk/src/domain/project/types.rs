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

// The conversions from and into the engine's project lookups are the crate's own: a public `From` impl would put the engine's types
// on the SDK's surface.
impl ProjectSearchScope {
    /// The search roots as the engine reads them.
    pub(crate) fn into_engine(
        self,
    ) -> ryo_language::domain::project_context::types::ProjectSearchScope {
        use ryo_language::domain::project_context::types::ProjectSearchScope as Engine;
        match self {
            Self::Undeclared => Engine::Undeclared,
            Self::Declared(roots) => Engine::Declared(
                roots
                    .into_iter()
                    .map(|root| ryo_language::domain::files::types::FilePath(root.0))
                    .collect(),
            ),
        }
    }
}

impl GivenProjectInventory {
    /// The given inventory as the engine reads it.
    pub(crate) fn into_engine(
        self,
    ) -> ryo_language::domain::project_context::types::GivenProjectInventory {
        use ryo_language::domain::project_context::types as engine;
        engine::GivenProjectInventory {
            entries: self
                .entries
                .into_iter()
                .map(|entry| engine::EntryObservation {
                    path: ryo_language::domain::files::types::FilePath(entry.path.0),
                    state: match entry.state {
                        ProjectEntryState::File => engine::ObservedEntryState::File,
                        ProjectEntryState::Directory => engine::ObservedEntryState::Directory,
                        ProjectEntryState::Other => engine::ObservedEntryState::Other,
                        ProjectEntryState::Missing => engine::ObservedEntryState::Missing,
                        ProjectEntryState::Unknown => engine::ObservedEntryState::Unknown,
                    },
                })
                .collect(),
            completeness: match self.completeness {
                ProjectInventoryCompleteness::Partial => {
                    engine::ProjectInventoryCompleteness::Partial
                }
                ProjectInventoryCompleteness::CompleteWithinDeclaredScope => {
                    engine::ProjectInventoryCompleteness::CompleteWithinDeclaredScope
                }
            },
        }
    }
}

impl ProjectDependency {
    /// A dependency the run examined, as the caller reads it.
    pub(crate) fn from_engine(
        dependency: ryo_core::inbound::use_cases::lint::types::ProjectDependency,
    ) -> Self {
        use ryo_core::inbound::use_cases::lint::types::ProjectDependencyState as Engine;
        Self {
            path: RelativePath(dependency.path.0),
            state: match dependency.state {
                Engine::Present(digest) => {
                    ProjectDependencyState::Present(DependencyFingerprint(digest.0))
                }
                Engine::Missing => ProjectDependencyState::Missing,
            },
        }
    }
}
