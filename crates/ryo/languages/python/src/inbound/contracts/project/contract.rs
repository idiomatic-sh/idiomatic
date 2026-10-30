//! The `project` contract: it implements the contract another crate declares. It says which files are the manifests it reads, asks the
//! manifest reading port what each declares, and answers, for every manifest relation of ryo's catalog, how Python reads it: each requirement a
//! package makes of another package of the repository, each package a workspace holds, and the package of each file, from what the
//! manifests declared; and, when the run asks for them, the file each import names and the files whose imports were examined, from
//! the notes the parse left of each file.

use std::sync::Arc;

use ryo::languages::{
    self, ManifestFile, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest,
    Refused, Resolved, Support,
};

use crate::domain::fact_notes::types::Note;
use crate::domain::links::service::linked;
use crate::domain::links::types::Linked;
use crate::domain::manifests::collections::manifests::analyzed_source::item::AnalyzedSourceItem;
use crate::domain::manifests::collections::manifests::file_package::item::FilePackageItem;
use crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem;
use crate::domain::manifests::collections::manifests::source_dependency::item::SourceDependencyItem;
use crate::domain::manifests::collections::manifests::workspace_member::item::WorkspaceMemberItem;
use crate::domain::merging::service::merged;
use crate::domain::packages::types::{ManifestPath, Package, Packages, Reason as Said, Refusal};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;
use crate::outbound::notes::interface::NotesPort;

/// `project`, implementing `ryo::languages::Project`: it reads each manifest with the adapters that claim it, and the manifests it
/// reads are the union of what the adapters claim.
pub(crate) struct ProjectContract {
    /// Builds the manifest adapters for one run, each reading the files it claims, over every manifest of the run's repository.
    projects: fn(&[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>>,
    /// The manifest file names the adapters claim, together.
    names: &'static [ManifestName],
    /// What reads the notes the parse left of each file.
    notes: Arc<dyn NotesPort>,
}

impl ProjectContract {
    /// `project` over the ports it calls, which the edge injects: what builds the adapters of a run, one for each package manager, and
    /// the notes.
    #[must_use]
    pub(crate) fn new(
        projects: fn(&[ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        let mut names: Vec<ManifestName> = Vec::new();
        for name in projects(&[]).iter().flat_map(|project| project.manifests()) {
            if !names.contains(name) {
                names.push(*name);
            }
        }
        Self {
            projects,
            names: Box::leak(names.into_boxed_slice()),
            notes,
        }
    }

    /// What the manifests of the request declare: the packages, and the manifests the tools refused.
    fn packages(&self, request: &ProjectRequest) -> Packages {
        let projects = (self.projects)(&request.manifests);
        let mut found = Vec::new();
        let mut refused: Vec<Refusal> = Vec::new();
        for manifest in &request.manifests {
            for project in projects
                .iter()
                .filter(|project| project.claims(&manifest.path) == Claim::Reads)
            {
                match project.declared(manifest) {
                    Ok(declared) => found.push(Package {
                        manifest: ManifestPath(manifest.path.0.clone()),
                        declared,
                    }),
                    Err(error) => {
                        if !refused
                            .iter()
                            .any(|each| each.manifest.0 == manifest.path.0)
                        {
                            refused.push(Refusal {
                                manifest: ManifestPath(manifest.path.0.clone()),
                                reason: Said(error.to_string().into()),
                            });
                        }
                    }
                }
            }
        }
        refused.sort_by(|left, right| left.manifest.cmp(&right.manifest));
        Packages {
            packages: merged(found),
            refused,
        }
    }
}

impl Project for ProjectContract {
    type Derived = Linked;

    fn manifests(&self) -> &'static [ManifestName] {
        self.names
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<Linked>, ProjectError> {
        let packages = self.packages(request);
        let refused = packages
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
            let notes: Vec<Option<Note>> = request
                .notes
                .iter()
                .map(|bytes| {
                    // ignore: idiomatic.arch/no-ok-on-result - the one error of a notes read is bytes that hold no note: a file the parse noted nothing of
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

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, Linked>> {
        languages::reads_manifests::<FilePackageItem>()
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, Linked>> {
        languages::reads_manifests::<AnalyzedSourceItem>()
    }
}
