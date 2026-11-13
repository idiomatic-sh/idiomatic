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
use crate::domain::manifests::collections::manifests::project_namespace_membership::item::ProjectNamespaceMembershipItem;
use crate::domain::manifests::collections::manifests::source_dependency::item::SourceDependencyItem;
use crate::domain::manifests::collections::manifests::workspace_member::item::WorkspaceMemberItem;
use crate::domain::merging::service::merged;
use crate::domain::packages::types::{ManifestPath, Package, Packages, Reason as Said, Refusal};
use crate::domain::project_namespace::types::{EntryShape, Presence};
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::manifest_reading::types::Claim;
use crate::outbound::notes::interface::NotesPort;

/// `project`, implementing `ryo::languages::Project`: it reads each manifest with the adapters that claim it, and the manifests it
/// reads are the union of what the adapters claim.
pub(crate) struct ProjectContract {
    /// Package context lookup with explicit Disk/Given isolation.
    module_context: Arc<dyn crate::outbound::module_context::interface::ModuleContextPort>,
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
        module_context: Arc<dyn crate::outbound::module_context::interface::ModuleContextPort>,
    ) -> Self {
        let mut names: Vec<ManifestName> = Vec::new();
        for name in projects(&[]).iter().flat_map(|project| project.manifests()) {
            if !names.contains(name) {
                names.push(*name);
            }
        }
        Self {
            module_context,
            projects,
            names: Box::leak(names.into_boxed_slice()),
            notes,
        }
    }

    /// What the manifests of the request declare: the packages, and the manifests the tools refused.
    fn packages(&self, request: &ProjectRequest) -> Linked {
        use crate::domain::manifest_validation::types::{
            FileValidation, ManifestValidations, MetadataDemand,
        };
        let projection_wanted = request.wanted.iter().any(|relation| {
            matches!(
                relation,
                languages::Relation::PackageDependency
                    | languages::Relation::WorkspaceMember
                    | languages::Relation::SourceDependency
                    | languages::Relation::FilePackage
                    | languages::Relation::AnalyzedSource
            )
        });
        let diagnostics_wanted = request
            .wanted
            .contains(&languages::Relation::ManifestDiagnostic);
        let demand = match (projection_wanted, diagnostics_wanted) {
            (true, true) => MetadataDemand::Both,
            (true, false) => MetadataDemand::Projection,
            (false, true) => MetadataDemand::Diagnostics,
            (false, false) => return Linked::default(),
        };
        let mut validation = ManifestValidations::default();
        let projects = (self.projects)(&request.manifests);
        let mut found = Vec::new();
        let mut refused: Vec<Refusal> = Vec::new();
        for manifest in &request.manifests {
            for project in projects
                .iter()
                .filter(|project| project.claims(&manifest.path) == Claim::Reads)
            {
                let primary_demand = if request.files.contains(&manifest.path) {
                    demand
                } else if projection_wanted {
                    MetadataDemand::Projection
                } else {
                    continue;
                };
                let reading = project.read(manifest, primary_demand);
                if let Some(result) = reading.validation {
                    if let Some(index) =
                        request.files.iter().position(|file| file == &manifest.path)
                    {
                        if let Ok(index) = u32::try_from(index) {
                            validation.files.push(FileValidation {
                                file: languages::FileId(index),
                                validation: result,
                            });
                        } else {
                            validation.incomplete.push(Said(
                                crate::domain::packages::constants::FILE_IDENTITY_EXCEEDS_CATALOG
                                    .into(),
                            ));
                        }
                    }
                }
                match reading.declaration {
                    Some(Ok(declared)) => found.push(Package {
                        manifest: ManifestPath(manifest.path.0.clone()),
                        declared,
                    }),
                    Some(Err(error)) => {
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
                    None => {}
                }
            }
        }
        if diagnostics_wanted {
            for (index, file) in request.files.iter().enumerate() {
                if file.0.rsplit('/').next() == Some(crate::domain::packages::constants::PYPROJECT)
                    && !validation
                        .files
                        .iter()
                        .any(|observed| {
                            matches!(usize::try_from(observed.file.0), Ok(position) if position == index)
                        })
                {
                    validation.incomplete.push(Said(
                        format!(
                            "{}{}",
                            crate::domain::packages::constants::VALIDATOR_DID_NOT_OBSERVE,
                            file.0
                        )
                        .into(),
                    ));
                }
            }
        }
        refused.sort_by(|left, right| left.manifest.cmp(&right.manifest));
        Linked {
            packages: Packages {
                packages: merged(found),
                refused,
            },
            manifest_validation: validation,
            ..Linked::default()
        }
    }
}

impl Project for ProjectContract {
    type Derived = Linked;

    fn manifests(&self) -> &'static [ManifestName] {
        self.names
    }

    fn primary_role(&self, path: languages::SourcePath<'_>) -> languages::PrimaryFileRole {
        crate::domain::language::service::primary_role(path, self.manifests())
    }

    fn project_entries(&self, request: &ProjectRequest, entries: &mut languages::ProjectEntries) {
        if !request
            .wanted
            .contains(&languages::Relation::ProjectNamespaceMembership)
        {
            return;
        }
        let notes: Vec<Option<Note>> = request
            .notes
            .iter()
            .map(|bytes| {
                let Ok(note) = self.notes.note_in(bytes) else {
                    return None;
                };
                Some(note)
            })
            .collect();
        let roots = crate::domain::project_namespace::service::imported_roots(&notes);
        entries.extend(crate::domain::project_namespace::service::candidates(
            languages::search_roots(&request.search_scope),
            &roots,
        ));
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<Linked>, ProjectError> {
        let Linked {
            packages,
            manifest_validation: validation,
            ..
        } = self.packages(request);
        // Refusals describe only the dependency outputs that were actually demanded and read.
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
        let mut derived = if links_wanted {
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
            linked(&request.files, &notes, packages)
        } else {
            Linked {
                packages,
                ..Linked::default()
            }
        };
        if request
            .wanted
            .contains(&languages::Relation::ModuleIdentity)
        {
            derived.module_context = self.module_context.resolve(request)?;
        }
        derived.manifest_validation = validation;
        if request
            .wanted
            .contains(&languages::Relation::ProjectNamespaceMembership)
        {
            let mut incomplete_notes = Vec::new();
            let notes: Vec<Option<Note>> = request
                .notes
                .iter()
                .enumerate()
                .map(|(number, bytes)| match self.notes.note_in(bytes) {
                    Ok(note) => Some(note),
                    Err(_) => {
                        if !bytes.0.is_empty() {
                            incomplete_notes
                                .push(languages::FileId(u32::try_from(number).unwrap_or(u32::MAX)));
                        }
                        None
                    }
                })
                .collect();
            let roots = crate::domain::project_namespace::service::imported_roots(&notes);
            derived.namespace_memberships = crate::domain::project_namespace::service::memberships(
                languages::search_roots(&request.search_scope),
                &roots,
                incomplete_notes,
                |path, shape| {
                    match shape {
                        EntryShape::File => languages::entry_state(&request.observed_entries, path)
                            .map(|state| state == languages::ObservedEntryState::File),
                        EntryShape::Directory => {
                            languages::entry_state(&request.observed_entries, path)
                                .map(|state| state == languages::ObservedEntryState::Directory)
                        }
                    }
                    .map(Presence::from)
                },
            );
        }
        let consulted = derived.module_context.consulted.clone();
        Ok(Resolved {
            refused,
            unlisted: Vec::new(),
            consulted,
            observed_entries: request.observed_entries.0.to_vec(),
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

    fn manifest_diagnostic(
        &self,
    ) -> Support<ManifestReader<languages::ManifestDiagnostic, Linked>> {
        languages::reads_manifests::<crate::domain::manifests::collections::manifests::manifest_diagnostic::item::ManifestDiagnosticItem>()
    }

    fn module_identity(&self) -> Support<ManifestReader<languages::ModuleIdentity, Linked>> {
        languages::reads_manifests::<crate::domain::manifests::collections::manifests::module_identity::item::ModuleIdentityItem>()
    }

    fn project_namespace_membership(
        &self,
    ) -> Support<ManifestReader<languages::ProjectNamespaceMembership, Linked>> {
        languages::reads_manifests::<ProjectNamespaceMembershipItem>()
    }
}
