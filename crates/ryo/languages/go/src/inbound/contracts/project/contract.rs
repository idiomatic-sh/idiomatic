//! The `project` contract: it implements the contract another crate declares. It says which files are the manifests it reads, asks the
//! manifest reading port what each declares, and answers, for every manifest relation of ryo's catalog, how Go reads it: each requirement a
//! module makes of another module of the repository, each module a workspace uses, and the module of each file, from what the
//! manifests declared; and, when the run asks for them, the files each import names and the files whose imports were examined, from
//! the notes the parse left of each file.

use std::sync::Arc;

use ryo::languages::{
    self, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest, Refused, Resolved,
    Support,
};

use crate::domain::fact_notes::types::Note;
use crate::domain::links::service::linked;
use crate::domain::links::types::Linked;
use crate::domain::manifests::collections::manifests::analyzed_source::item::AnalyzedSourceItem;
use crate::domain::manifests::collections::manifests::file_package::item::FilePackageItem;
use crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem;
use crate::domain::manifests::collections::manifests::source_dependency::item::SourceDependencyItem;
use crate::domain::manifests::collections::manifests::workspace_member::item::WorkspaceMemberItem;
use crate::domain::modules::constants::{MODULE_FILE, WORKSPACE_FILE};
use crate::domain::modules::service::{manifestness_of, sorted};
use crate::domain::modules::types::{
    Manifest, ManifestPath, Manifestness, Modules, Reason as Said, Refusal,
};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::interface::NotesPort;

/// The manifests the language reads.
const MANIFESTS: &[ManifestName] = &[ManifestName(MODULE_FILE), ManifestName(WORKSPACE_FILE)];

/// `project`, implementing `ryo::languages::Project`.
pub(crate) struct ProjectContract {
    /// The manifest reading port the contract reads the repository's manifests through.
    project: Arc<dyn ManifestReadingPort>,
    /// The notes port the contract reads the notes the parse left of each file through.
    notes: Arc<dyn NotesPort>,
}

impl ProjectContract {
    /// `project` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(project: Arc<dyn ManifestReadingPort>, notes: Arc<dyn NotesPort>) -> Self {
        Self { project, notes }
    }

    /// The notes the parse left of each file of the request, in the order of its files: none for a file it left none of, or whose
    /// bytes are no note.
    fn notes_of(&self, request: &ProjectRequest) -> Vec<Option<Note>> {
        request
            .notes
            .iter()
            .map(|bytes| {
                if bytes.0.is_empty() {
                    None
                } else {
                    // the one error of a notes read is bytes that hold no note: a file the parse noted nothing of
                    let Ok(note) = self.notes.note_in(bytes) else {
                        return None;
                    };
                    Some(note)
                }
            })
            .collect()
    }

    /// What the manifests of the request declare: the modules and workspaces, and the manifests the tool refused.
    fn modules(&self, request: &ProjectRequest) -> Modules {
        let mut found = Vec::new();
        let mut refused = Vec::new();
        for manifest in &request.manifests {
            let kind = manifestness_of(&manifest.path);
            if kind == Manifestness::Other {
                continue;
            }
            match self.project.declared(kind, &manifest.text) {
                Ok(declared) => found.push(Manifest {
                    manifest: ManifestPath(manifest.path.0.clone()),
                    declared,
                }),
                Err(error) => refused.push(Refusal {
                    manifest: ManifestPath(manifest.path.0.clone()),
                    reason: Said(error.to_string().into()),
                }),
            }
        }
        refused.sort_by(|left, right| left.manifest.cmp(&right.manifest));
        Modules {
            manifests: sorted(found),
            refused,
        }
    }
}

impl Project for ProjectContract {
    type Derived = Linked;

    fn manifests(&self) -> &'static [ManifestName] {
        MANIFESTS
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
        let modules = self.modules(request);
        let refused = modules
            .refused
            .iter()
            .map(|each| Refused {
                path: languages::FilePath(each.manifest.0.clone()),
                reason: languages::ToolMessage(each.reason.0.clone()),
            })
            .collect();
        let links_wanted = request.wanted.iter().any(|each| {
            matches!(
                each,
                languages::Relation::SourceDependency | languages::Relation::AnalyzedSource
            )
        });
        let derived = if links_wanted {
            linked(&request.files, &self.notes_of(request), modules)
        } else {
            Linked {
                modules,
                ..Linked::default()
            }
        };
        Ok(Resolved {
            observed_entries: request.observed_entries.0.to_vec(),
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

    fn source_dependency(&self) -> Support<ManifestReader<languages::SourceDependency, Linked>> {
        languages::reads_manifests::<SourceDependencyItem>()
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

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, Linked>> {
        languages::reads_manifests::<AnalyzedSourceItem>()
    }
}
