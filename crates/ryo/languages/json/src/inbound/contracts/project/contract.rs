//! The `project` contract: the language's implementation of `ryo::languages::Project`. It names no manifest and reads nothing, and answers
//! every manifest relation of ryo's catalog as not in the language. A language with manifests lists their names in `manifests`, reads
//! them in `resolve` and answers the relations its readers write with `languages::reads`.

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
        // standard: RFC 8259 (STD 90), section 2: a JSON text is one value, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, ()>> {
        // standard: RFC 8259 (STD 90), section 2: a JSON text is one value, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, ()>> {
        // standard: RFC 8259 (STD 90), section 2: a JSON text is one value, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, ()>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
    }

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, ()>> {
        // standard: RFC 8259 (STD 90), section 2: a JSON text is one value, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, ()>> {
        // standard: RFC 8259 (STD 90), section 2: a JSON text is one value, and the standard names no project manifest.
        languages::absent(AbsenceCause::NoManifests)
    }
}
