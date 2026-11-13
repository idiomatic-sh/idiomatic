//! The `project` contract: it implements the contract another crate declares. CSS has no manifest of its own: a stylesheet is read
//! where it stands, and what it imports is a `static_import` of its tree. It names no manifest, reads nothing, and answers every
//! manifest relation of ryo's catalog as not in the language, for that reason.

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
        languages::primary_role_from_manifests(path, self.manifests().iter().copied())
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut ryo::languages::ProjectEntries,
    ) {
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<()>, ProjectError> {
        Ok(Resolved {
            observed_entries: request.observed_entries.0.to_vec(),
            refused: Vec::new(),
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: (),
        })
    }

    fn package_dependency(&self) -> Support<ManifestReader<languages::PackageDependency, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest; it names no package, so no dependency of one.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest; it has no workspace of packages, so no member of one.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest; it names no package manager, so no dependency on a source of one.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn manifest_diagnostic(&self) -> Support<ManifestReader<languages::ManifestDiagnostic, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest.
        languages::absent(languages::AbsenceCause::NoManifests)
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn project_namespace_membership(
        &self,
    ) -> Support<ManifestReader<languages::ProjectNamespaceMembership, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest.
        languages::absent(languages::AbsenceCause::NoManifests)
    }

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest; it has no package a file belongs to.
        languages::absent(AbsenceCause::NoManifests)
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, ()>> {
        // standard: the CSS Syntax Module Level 3, §3 Tokenizing and Parsing CSS: a stylesheet is parsed from its text alone and the standard defines no manifest; it has no manifest that lists the sources of a package.
        languages::absent(AbsenceCause::NoManifests)
    }
}
