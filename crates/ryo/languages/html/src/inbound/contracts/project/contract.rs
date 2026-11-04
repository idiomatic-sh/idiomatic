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

    fn primary_role(&self, path: languages::SourcePath<'_>) -> languages::PrimaryFileRole {
        languages::primary_role_from_manifests(
            path,
            self.manifests().iter().map(|manifest| manifest.0),
        )
    }

    fn project_entries(&self, _request: &ProjectRequest) -> Vec<ryo::languages::FilePath> {
        Vec::new()
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
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document trees and no package manifest or dependency graph.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document trees and no package manifest or dependency graph.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document trees and no package manifest or dependency graph.
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
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document trees and no package manifest or dependency graph.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, ()>> {
        // standard: WHATWG HTML, Parsing HTML documents: https://html.spec.whatwg.org/multipage/parsing.html defines document trees and no package manifest or dependency graph.
        languages::absent(AbsenceCause::NoManifests)
    }
}
