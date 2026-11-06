//! The `project` contract: it implements the contract another crate declares. Markdown is a format of
//! documents with no project of its own: a Markdown file is read where it stands. It names no manifest, reads nothing, and
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

    fn primary_role(&self, path: languages::SourcePath<'_>) -> languages::PrimaryFileRole {
        languages::primary_role_from_manifests(
            path,
            self.manifests().iter().map(|manifest| manifest.0),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut Vec<ryo::languages::FilePath>,
    ) {
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<()>, ProjectError> {
        Ok(Resolved {
            observed_entries: request.observed_entries.receipt(),
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    fn package_dependency(&self) -> Support<ManifestReader<languages::PackageDependency, ()>> {
        // standard: mdast.md and micromark-util-types.d.ts: neither defines a project manifest; a Markdown file is a document, not a package.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, ()>> {
        // standard: mdast.md and micromark-util-types.d.ts: neither defines a project manifest; a Markdown file is a document, not a package.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, ()>> {
        // standard: mdast.md and micromark-util-types.d.ts: neither defines a project manifest; a Markdown file is a document, not a package.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn manifest_diagnostic(&self) -> Support<ManifestReader<languages::ManifestDiagnostic, ()>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, ()>> {
        languages::absent(AbsenceCause::NotYetRead)
    }

    fn project_namespace_membership(
        &self,
    ) -> Support<ManifestReader<languages::ProjectNamespaceMembership, ()>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
    }

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, ()>> {
        // standard: mdast.md and micromark-util-types.d.ts: neither defines a project manifest; a Markdown file is a document, not a package.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, ()>> {
        // standard: mdast.md and micromark-util-types.d.ts: neither defines a project manifest; a Markdown file is a document, not a package.
        languages::absent(AbsenceCause::NoManifests)
    }
}
