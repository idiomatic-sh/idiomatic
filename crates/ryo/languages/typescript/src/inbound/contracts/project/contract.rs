//! The project contract resolves package manifests and configuration references, and, when the run asks for them, the file each
//! import names and the files whose imports were examined, from the notes the parse left of each file.

use crate::domain::catalogs::service::resolved;
use crate::domain::fact_notes::types::Note;
use crate::domain::links::service::linked;
use crate::domain::links::types::Linked;
use crate::domain::manifests::collections::manifests::{
    analyzed_source::item::AnalyzedSourceItem, file_package::item::FilePackageItem,
    package_dependency::item::PackageDependencyItem, source_dependency::item::SourceDependencyItem,
    workspace_member::item::WorkspaceMemberItem,
};
use crate::domain::packages::constants::SEPARATOR;
use crate::domain::packages::service::assembled;
use crate::domain::packages::types::{Fact, ManifestPath, Packages};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::interface::NotesPort;
use ryo::languages::{
    self, FilePath, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest, Resolved,
    Support,
};
use std::sync::Arc;

/// `project`, implementing `ryo::languages::Project`: the repository's packages, as every manifest adapter reads them.
pub(crate) struct ProjectContract {
    /// Builds the manifest adapters of one run over the manifests of that run, each reading the files it claims: the edge composes them.
    adapters: fn(&[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>>,
    /// What reads back the notes the parse wrote of each file.
    notes: Arc<dyn NotesPort>,
    /// The manifest file names the adapters claim, together.
    names: &'static [ManifestName],
}

impl ProjectContract {
    /// `project` over what builds the adapters that read the manifests of a run, and the port the notes are read back with: it names
    /// every manifest file any of the adapters claims, once. What an adapter claims is the same whatever run it is built for.
    #[must_use]
    pub(crate) fn new(
        adapters: fn(&[FilePath]) -> Vec<Arc<dyn ManifestReadingPort>>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        let mut names: Vec<ManifestName> = adapters(&[])
            .iter()
            .flat_map(|adapter| adapter.claims().iter().copied())
            .collect();
        names.sort_by_key(|name| name.0);
        names.dedup_by_key(|name| name.0);
        Self {
            adapters,
            notes,
            names: Box::leak(names.into_boxed_slice()),
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
            self.manifests().iter().map(|manifest| manifest.0),
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
        // The adapters of this run, over its manifests: member globs select from them.
        let adapters = (self.adapters)(&candidates);
        let mut facts = Vec::new();
        for manifest in &request.manifests {
            let file = manifest.path.0.rsplit(SEPARATOR).next();
            for adapter in adapters
                .iter()
                .filter(|adapter| adapter.claims().iter().any(|name| Some(name.0) == file))
            {
                match adapter.declared(manifest) {
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
        let derived = if links_wanted {
            // A file the parse left no notes of, or notes that are not a TypeScript file's, links to nothing.
            let notes: Vec<Option<Note>> = request
                .notes
                .iter()
                .map(|bytes| {
                    // the one error of a notes read is bytes that hold no note: a file the parse noted nothing of
                    self.notes.note_in(bytes).ok()
                })
                .collect();
            linked(&request.files, &notes, packages)
        } else {
            Linked {
                packages,
                ..Linked::default()
            }
        };
        Ok(Resolved {
            observed_entries: request.observed_entries.receipt(),
            refused,
            unlisted: Vec::new(),
            consulted: Vec::new(),
            derived,
        })
    }

    fn package_dependency(&self) -> Support<ManifestReader<languages::PackageDependency, Linked>> {
        languages::reads_manifests::<PackageDependencyItem>()
    }
    fn workspace_member(&self) -> Support<ManifestReader<languages::WorkspaceMember, Linked>> {
        languages::reads_manifests::<WorkspaceMemberItem>()
    }
    fn manifest_diagnostic(
        &self,
    ) -> Support<ManifestReader<languages::ManifestDiagnostic, Linked>> {
        languages::absent(languages::AbsenceCause::NotYetRead)
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
    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, Linked>> {
        languages::reads_manifests::<SourceDependencyItem>()
    }
    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, Linked>> {
        languages::reads_manifests::<AnalyzedSourceItem>()
    }
}
