//! The `project` contract: the language's implementation of `ryo::languages::Project`. It reads the package manifests npm, pnpm and
//! Bun write, and, when the run asks for them, links each import to the file of the repository it names and says which files' imports
//! were examined, from the notes the parse left of each file.

use crate::domain::catalogs::service::resolved;
use crate::domain::fact_notes::types::Note;
use crate::domain::links::service::linked;
use crate::domain::links::types::Linked;
use crate::domain::manifests::collections::manifests::{
    analyzed_source::item::AnalyzedSourceItem, file_package::item::FilePackageItem,
    manifest_diagnostic::item::ManifestDiagnosticItem,
    package_dependency::item::PackageDependencyItem, source_dependency::item::SourceDependencyItem,
    workspace_member::item::WorkspaceMemberItem,
};
use crate::domain::packages::constants::SEPARATOR;
use crate::domain::packages::service::assembled;
use crate::domain::packages::types::{Fact, ManifestPath, Packages};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::interface::NotesPort;
use ryo::languages::{
    self, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest, Resolved, Support,
};
use std::sync::Arc;

/// `project`, implementing `ryo::languages::Project`: the repository's packages, as every manifest adapter reads them.
pub(crate) struct ProjectContract {
    /// The manifest adapters, each reading the files it claims.
    adapters: Vec<Arc<dyn ManifestReadingPort>>,
    /// The manifest file names the adapters claim, together.
    names: &'static [ManifestName],
    /// What reads the notes the parse left of each file.
    notes: Arc<dyn NotesPort>,
}

impl ProjectContract {
    /// `project` over the adapters that read manifests, and the port that reads the notes of the files: it names every manifest file any
    /// of the adapters claims, once.
    #[must_use]
    pub(crate) fn new(
        adapters: Vec<Arc<dyn ManifestReadingPort>>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        let mut names: Vec<ManifestName> = adapters
            .iter()
            .flat_map(|adapter| adapter.claims().iter().copied())
            .collect();
        names.sort_by_key(|name| name.0);
        names.dedup_by_key(|name| name.0);
        Self {
            adapters,
            names: Box::leak(names.into_boxed_slice()),
            notes,
        }
    }
}

impl Project for ProjectContract {
    type Derived = Linked;

    fn manifests(&self) -> &'static [ManifestName] {
        self.names
    }

    fn primary_role(&self, path: languages::SourcePath<'_>) -> languages::PrimaryFileRole {
        languages::primary_role_from_manifests(
            path,
            self.manifests().iter().copied(),
        )
    }

    fn project_entries(
        &self,
        _request: &ProjectRequest,
        _entries: &mut Vec<ryo::languages::FilePath>,
    ) {
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<Linked>, ProjectError> {
        let mut packages = Packages::default();
        let candidates = request
            .manifests
            .iter()
            .map(|manifest| manifest.path.clone())
            .collect::<Vec<_>>();
        let mut facts = Vec::new();
        for manifest in &request.manifests {
            let file = manifest.path.0.rsplit(SEPARATOR).next();
            let claiming: Vec<_> = self
                .adapters
                .iter()
                .filter(|adapter| adapter.claims().iter().any(|name| Some(name.0) == file))
                .collect();
            if claiming.is_empty() {
                continue;
            }
            for adapter in claiming {
                match adapter.declared(manifest, &candidates) {
                    Ok(declared) => facts.push(Fact {
                        manifest: ManifestPath(manifest.path.0.clone()),
                        about: adapter.about(manifest),
                        declared,
                    }),
                    // Adapters that read one file alike refuse it alike: it is refused once.
                    Err(error) => {
                        if !packages
                            .refused
                            .iter()
                            .any(|refused| refused.path == manifest.path)
                        {
                            packages.refused.push(languages::Refused {
                                path: manifest.path.clone(),
                                reason: languages::ToolMessage(error.to_string().into()),
                            });
                        }
                    }
                }
            }
        }
        packages.entries = assembled(facts);
        let unresolved = resolved(&mut packages);
        packages.refused.extend(unresolved);
        packages
            .entries
            .sort_by(|left, right| left.manifest.cmp(&right.manifest));
        packages
            .refused
            .sort_by(|left, right| left.path.0.cmp(&right.path.0));
        let refused = packages.refused.clone();
        let links_wanted = request.wanted.iter().any(|each| {
            matches!(
                each,
                languages::Relation::SourceDependency | languages::Relation::AnalyzedSource
            )
        });
        let links = if links_wanted {
            let notes: Vec<Option<Note>> = request
                .notes
                .iter()
                .map(|bytes| {
                    // the one error of a notes read is bytes that hold no note: a file the parse noted nothing of
                    let Ok(note) = self.notes.note_in(bytes) else {
                        return None;
                    };
                    Some(note)
                })
                .collect();
            linked(&request.files, &notes)
        } else {
            Linked::default()
        };
        Ok(Resolved {
            observed_entries: request.observed_entries.0.to_vec(),
            refused,
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived: Linked { packages, ..links },
        })
    }

    fn package_dependency(&self) -> Support<ManifestReader<languages::PackageDependency, Linked>> {
        languages::reads_manifests::<PackageDependencyItem>()
    }

    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, Linked>> {
        languages::reads_manifests::<WorkspaceMemberItem>()
    }

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, Linked>> {
        languages::reads_manifests::<SourceDependencyItem>()
    }

    fn manifest_diagnostic(
        &self,
    ) -> Support<ManifestReader<languages::ManifestDiagnostic, Linked>> {
        languages::reads_manifests::<ManifestDiagnosticItem>()
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, Linked>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
    }

    fn project_namespace_membership(
        &self,
    ) -> Support<ManifestReader<languages::ProjectNamespaceMembership, Linked>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
    }

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, Linked>> {
        languages::reads_manifests::<FilePackageItem>()
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, Linked>> {
        languages::reads_manifests::<AnalyzedSourceItem>()
    }
}
