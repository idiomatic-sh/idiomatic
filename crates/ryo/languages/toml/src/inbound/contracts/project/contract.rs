//! The `project` contract: it implements the contract another crate declares. TOML is a format of
//! data with no project of its own: a TOML file is read where it stands, and the manifests written
//! in TOML belong to the language whose tool reads them. It names no manifest, reads nothing, and
//! answers every manifest relation of ryo's catalog as not in the language, for that reason.

use ryo::languages::{
    self, AbsenceCause, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest,
    Resolved, Support,
};

/// `project`, implementing `ryo::languages::Project`.
pub(crate) struct ProjectContract;

impl ProjectContract {
    /// `project`, which calls no port.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Default for ProjectContract {
    fn default() -> Self {
        Self::new()
    }
}

impl Project for ProjectContract {
    type Derived = ();

    fn manifests(&self) -> &'static [ManifestName] {
        &[]
    }

    fn resolve(&self, _request: &ProjectRequest) -> Result<Resolved<()>, ProjectError> {
        Ok(Resolved {
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    fn package_dependency(&self) -> Support<ManifestReader<languages::PackageDependency, ()>> {
        // standard: toml.abnf (TOML 1.1.0), the rule `toml`: a document is expressions of keys, tables and comments, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, ()>> {
        // standard: toml.abnf (TOML 1.1.0), the rule `toml`: a document is expressions of keys, tables and comments, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, ()>> {
        // standard: toml.abnf (TOML 1.1.0), the rule `toml`: a document is expressions of keys, tables and comments, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, ()>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, ()>> {
        // standard: toml.abnf (TOML 1.1.0), the rule `toml`: a document is expressions of keys, tables and comments, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, ()>> {
        // standard: toml.abnf (TOML 1.1.0), the rule `toml`: a document is expressions of keys, tables and comments, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }
}
