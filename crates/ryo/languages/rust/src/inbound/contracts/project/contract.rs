//! The `project` contract: it implements the contract another crate declares. It says which files are
//! the manifests it reads, asks the project port what the repository's manifests declare, and answers, for every
//! manifest relation of ryo's catalog, how Rust reads it: each package dependency and each workspace member
//! from what the manifests declared, and, when the run asks for them, how the source files refer to one another and
//! which were examined, from the notes the parse left of each. No relation is left unanswered; a relation added to
//! the catalog stops this from compiling until it is.

use crate::domain::paths::service::present;
use crate::domain::paths::types::Matching;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use ryo::languages::{
    self, ManifestFile, ManifestName, ManifestReader, Project, ProjectError, ProjectRequest,
    Refused, Resolved, RootPath, Support,
};

use crate::domain::links::collections::links::analyzed_source::item::AnalyzedSourceItem;
use crate::domain::links::collections::links::file_package::item::FilePackageItem;
use crate::domain::links::collections::links::source_dependency::item::SourceDependencyItem;
use crate::domain::links::service::{declared_in, draw, examine, facts_in, gaps_in, merged};
use crate::domain::links::types::{Linked, Run};
use crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem;
use crate::domain::manifests::collections::manifests::workspace_member::item::WorkspaceMemberItem;
use crate::domain::module_tree::types::Segment;
use crate::domain::package_index::constants::{MANIFEST, MANIFESTS};
use crate::domain::package_index::service::{index, kept_files};
use crate::domain::package_index::types::ListIndex;
use crate::domain::packages::constants::{CONFIGURATION, LEGACY_CONFIGURATION};
use crate::domain::packages::service::{read, unlisted_among};
use crate::domain::packages::types::Packages;
use crate::domain::paths::service::{entries_among, found_above, name_of};
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::paths::types::{FileText, Settlement};
use crate::domain::targets::types::Manifest as TargetManifest;
use crate::domain::workspaces::service::settled;
use crate::domain::workspaces::types::{Configured, Looking, Reason};
use crate::outbound::conditions::interface::ConditionsPort;
use crate::outbound::job_scheduling::interface::JobSchedulingPort;
use crate::outbound::job_scheduling::types::{JobIndex, ScheduledJobCount};
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::notes::errors::NotesError;
use crate::outbound::notes::interface::NotesPort;
use crate::outbound::pattern_matching::interface::PatternMatchingPort;
use crate::outbound::repository::interface::RepositoryPort;

/// Builds, for the root of a run, the repository port over it.
type RepositoryAt = Arc<dyn Fn(&RootPath) -> Box<dyn RepositoryPort> + Send + Sync>;

/// `project`, implementing `ryo::languages::Project`.
pub(crate) struct ProjectContract {
    /// The reader of each `Cargo.toml` the project holds.
    manifest: Arc<dyn ManifestPort>,
    /// Builds, for the root of a run, the repository's files and directories, as the walk lists them.
    repository: RepositoryAt,
    /// The glob matcher for workspace members and excludes.
    matching: Arc<dyn PatternMatchingPort>,
    /// The workers that read the parsed files' notes in parallel.
    scheduler: Arc<dyn JobSchedulingPort>,
    /// The evaluator of `cfg` conditions on platform-specific dependencies.
    conditions: Arc<dyn ConditionsPort>,
    /// The store holding what each file's parse noted.
    notes: Arc<dyn NotesPort>,
}

impl ProjectContract {
    /// `project` over the ports it calls, which the edge injects.
    #[must_use]
    pub(crate) fn new(
        manifest: Arc<dyn ManifestPort>,
        repository: RepositoryAt,
        matching: Arc<dyn PatternMatchingPort>,
        scheduler: Arc<dyn JobSchedulingPort>,
        conditions: Arc<dyn ConditionsPort>,
        notes: Arc<dyn NotesPort>,
    ) -> Self {
        Self {
            manifest,
            repository,
            matching,
            scheduler,
            conditions,
            notes,
        }
    }

    /// Runs `each` over the items on the scheduler's workers and gathers what it answers in the order of the items.
    fn spread<T, R>(&self, items: &[T], each: &(dyn Fn(ListIndex, &T) -> R + Sync)) -> Vec<R>
    where
        T: Sync,
        R: Send,
    {
        let slots: Vec<Mutex<Option<R>>> = std::iter::repeat_with(|| Mutex::new(None))
            .take(items.len())
            .collect();
        self.scheduler.run(
            ScheduledJobCount(u32::try_from(items.len()).unwrap_or(u32::MAX)),
            &|job: JobIndex| {
                let at = usize::try_from(job.0).unwrap_or(usize::MAX);
                if let (Some(item), Some(slot)) = (items.get(at), slots.get(at)) {
                    let answer = each(ListIndex(at), item);
                    *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(answer);
                }
            },
        );
        slots
            .into_iter()
            .filter_map(|slot| slot.into_inner().unwrap_or_else(PoisonError::into_inner))
            .collect()
    }

    /// How the source files of the request refer to one another, from what the manifests declared and the notes the parse left of each:
    /// the names modules bear, the facts of each file, the trees of the packages and the examination of each file are each worked on
    /// in all the threads there are.
    fn linked(&self, request: &ProjectRequest, packages: &Packages) -> Linked {
        let files = &request.files;
        let kept = kept_files(files, &packages.outputs);
        let owners = index(&kept, &packages.roots);
        // Bytes that hold no note are a file the parse noted nothing of.
        let declared = self.spread(&request.notes, &|_, note| match self.notes.note_in(note) {
            Ok(each) => Some(declared_in(&each)),
            Err(NotesError::NotANote) => None,
        });
        let names: BTreeSet<Segment> = declared.into_iter().flatten().flatten().collect();
        let facts = self.spread(&request.notes, &|_, note| match self.notes.note_in(note) {
            Ok(each) => Some(facts_in(each, &names)),
            Err(NotesError::NotANote) => None,
        });
        let run = Run {
            kept: &kept,
            files,
            facts: &facts,
        };
        let drawn = self.spread(&owners.directories, &|_, package| draw(run, package));
        let trees: BTreeMap<_, _> = drawn
            .into_iter()
            .map(|each| (each.directory, each.trees))
            .collect();
        let examined = self.spread(files, &|place, file| {
            examine(run, &owners, &trees, place, file)
        });
        merged(run, gaps_in(&trees), examined)
    }

    /// What the manifests of the repository at `root` declare, read in process every time: it is a function of the files, and no process
    /// answers. The domain reads nothing: it works from what it has been told, wants what it was not told, and this reads what it
    /// wants, the supplied manifests first, and runs it again until it wants nothing. Each file it reads from the disk and finds there
    /// is put in `consulted`: the manifests besides those supplied, and the configurations found above and in the repository.
    pub(crate) fn packages(
        &self,
        root: &RootPath,
        supplied: &[ManifestFile],
        consulted: &mut Vec<RepoPath>,
    ) -> Packages {
        self.packages_in(&Section::Absent, root, supplied, consulted)
    }

    /// The same, read from the disk below `root`, or, when `given` lists the repository's files, from them and the supplied manifests
    /// alone: then nothing is read from the disk, nothing above the root is looked at, and nothing is consulted.
    fn packages_in(
        &self,
        given: &Section<&[languages::FilePath]>,
        root: &RootPath,
        supplied: &[ManifestFile],
        consulted: &mut Vec<RepoPath>,
    ) -> Packages {
        let repository = (self.repository)(root);
        let paths: Vec<RepoPath> = supplied
            .iter()
            .map(|manifest| RepoPath(manifest.path.0.clone()))
            .filter(|path| *name_of(path).0 == *MANIFEST)
            .collect();
        let mut looking = Looking::default();
        looking
            .manifests
            .seen
            .extend(supplied.iter().map(|manifest| {
                (
                    RepoPath(manifest.path.0.clone()),
                    self.parsed(&FileText(manifest.text.0.clone())),
                )
            }));
        loop {
            let packages = read(&paths, &mut looking);
            if settled(&looking) == Settlement::Settled {
                return packages;
            }
            looking.manifests.seen.extend(
                std::mem::take(&mut looking.manifests.wanted)
                    .into_iter()
                    .map(|path| {
                        let text = match given {
                            Section::Present(_) => Section::Absent,
                            Section::Absent => repository.text(&path),
                        };
                        if matches!(text, Section::Present(_)) {
                            consulted.push(path.clone());
                        }
                        let told = present(text)
                            .ok_or_else(|| Reason(path.0.clone()))
                            .and_then(|text| self.parsed(&text));
                        (path, told)
                    }),
            );
            looking.directories.seen.extend(
                std::mem::take(&mut looking.directories.wanted)
                    .into_iter()
                    .map(|directory| {
                        let entries = match given {
                            Section::Present(files) => entries_among(files, supplied, &directory),
                            Section::Absent => repository.entries(&directory).0,
                        };
                        (directory, entries)
                    }),
            );
            looking.matches.seen.extend(
                std::mem::take(&mut looking.matches.wanted)
                    .into_iter()
                    .map(|comparison| {
                        let matching = self
                            .matching
                            .matches(&comparison.pattern, &comparison.name)
                            .unwrap_or(Matching::Differs);
                        (comparison, matching)
                    }),
            );
            looking.configurations.seen.extend(
                std::mem::take(&mut looking.configurations.wanted)
                    .into_iter()
                    .map(|directory| {
                        let configured = match given {
                            Section::Present(_) => Vec::new(),
                            Section::Absent => {
                                self.configurations(&*repository, &directory, consulted)
                            }
                        };
                        (directory, configured)
                    }),
            );
            looking.platforms.seen.extend(
                std::mem::take(&mut looking.platforms.wanted)
                    .into_iter()
                    .map(|platform| {
                        let shown = self.conditions.show(&platform);
                        (platform, shown)
                    }),
            );
        }
    }

    /// What the text of a manifest declares, or why it cannot be read.
    fn parsed(&self, text: &FileText) -> Result<TargetManifest, Reason> {
        self.manifest
            .read(text)
            .map_err(|error| Reason(error.to_string().into()))
    }

    /// The configurations Cargo finds from `directory` up, nearest first: in each directory, `.cargo/config.toml`, else the legacy
    /// `.cargo/config`. Each one found is put in `consulted`, by its path from the root, and so is each one looked for and not found.
    fn configurations(
        &self,
        repository: &dyn RepositoryPort,
        directory: &RepoPath,
        consulted: &mut Vec<RepoPath>,
    ) -> Vec<Configured> {
        let current = repository
            .above(directory, &EntryName(CONFIGURATION.into()))
            .0;
        let legacy = repository
            .above(directory, &EntryName(LEGACY_CONFIGURATION.into()))
            .0;
        current
            .into_iter()
            .zip(legacy)
            .filter_map(|(modern, old)| {
                let (look, name) = if matches!(modern.text, Section::Present(_)) {
                    (modern, CONFIGURATION)
                } else {
                    // The newer name is looked for first: one that appears changes what the lookup finds, so it is consulted even
                    // when there is none, as is the older name when there is none of it.
                    consulted.push(found_above(
                        directory,
                        modern.height,
                        &EntryName(CONFIGURATION.into()),
                    ));
                    (old, LEGACY_CONFIGURATION)
                };
                consulted.push(found_above(directory, look.height, &EntryName(name.into())));
                let Section::Present(text) = &look.text else {
                    return None;
                };
                // A configuration that does not parse says nothing: the outputs go where the next one, or Cargo's default, puts them.
                let Ok(config) = self.manifest.configuration(text) else {
                    return None;
                };
                Some(Configured {
                    target_dir: match &config.target_dir {
                        Section::Present(written) => {
                            Section::Present(repository.placed(directory, look.height, written))
                        }
                        Section::Absent => Section::Absent,
                    },
                })
            })
            .collect()
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
            self.manifests().iter().map(|manifest| manifest.0),
        )
    }

    fn resolve(&self, request: &ProjectRequest) -> Result<Resolved<Linked>, ProjectError> {
        if !request
            .manifests
            .iter()
            .any(|manifest| *name_of(&RepoPath(manifest.path.0.clone())).0 == *MANIFEST)
        {
            return Ok(Resolved {
                refused: Vec::new(),
                unlisted: Vec::new(),
                consulted: Vec::new(),
                derived: Linked::default(),
            });
        }
        let mut found = Vec::new();
        let packages = match request.repository {
            languages::Repository::Disk => {
                self.packages(&request.root, &request.manifests, &mut found)
            }
            languages::Repository::Given => self.packages_in(
                &Section::Present(&request.files),
                &request.root,
                &request.manifests,
                &mut found,
            ),
        };
        found.sort_unstable();
        found.dedup();
        let consulted = found
            .into_iter()
            .map(|path| languages::FilePath(path.0))
            .collect();
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
            self.linked(request, &packages)
        } else {
            Linked::default()
        };
        let unlisted = unlisted_among(&request.files, &packages)
            .into_iter()
            .map(|manifest| languages::FilePath(manifest.0))
            .collect();
        derived.packages = Some(packages);
        Ok(Resolved {
            refused,
            unlisted,
            consulted,
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

    fn file_package(&self) -> Support<ManifestReader<languages::FilePackage, Linked>> {
        languages::reads_manifests::<FilePackageItem>()
    }

    fn analyzed_source(&self) -> Support<ManifestReader<languages::AnalyzedSource, Linked>> {
        languages::reads_manifests::<AnalyzedSourceItem>()
    }
}
