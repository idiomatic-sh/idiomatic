// ignore: idiomatic.arch/file-length - `add` is one use case: find the definition, resolve where it goes, draft the entry and relock, each step an orchestration over the ports the use case holds.
//! The `add` use case: the definition found in the workspace's packages, and its entry written.

use super::errors::AddError;
use super::interface::AddUseCase;
use super::types::{AddReport, AddRequest, Novelty, ShownEntry};
use crate::domain::captures::types::Interpolations;
use crate::domain::definitions::constants::{LOCK_FILE, MISSING_ROOT, WORKSPACE_FILE};
use crate::domain::definitions::service::{description, governed, local, locals, shared, unloaded};
use crate::domain::definitions::types::{LoadedWorkspace, LocalFile, Role};
use crate::domain::entries::service::{derived, literal_params, pin};
use crate::domain::files::types::FileText;
use crate::domain::loading::service::{begin, definition_loaded, finish, next};
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::service::{
    kind_directory, kind_file, latest, lock, locked_version, root_at,
};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, PackageRoot, Reach, Release, ResolvedPackage, SourcePath,
    VersionText,
};
use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::service::resolve as resolve_selectors;
use crate::domain::selectors::service::{covers_any, first_match};
use crate::domain::selectors::types::{
    Coverage, PathPattern, SelectionContext, Selector, Selectors, Spelling,
};
use crate::domain::variables::service::apply as apply_variables;
use crate::domain::writing::types::Writing;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{DefinitionText, EntryDraft};
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::interface::FoundRepository;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;
use std::collections::VecDeque;
use std::sync::Arc;

/// `add`, over the ports the edge injects.
pub struct AddUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
}

impl std::fmt::Debug for AddUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl AddUseCaseImpl {
    /// What the workspace's lock pins; none when there is no lock. A lock that does not read is an error to say, never a
    /// lock to treat as missing and rewrite.
    fn pins(
        &self,
        found: &dyn FoundRepository,
    ) -> Result<Option<crate::domain::packages::types::Lock>, AddError> {
        let repository = found;
        let lock_file = join(found.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
        // Only a lock that is not there is none: one that is there and does not read fails, never a lock to rewrite.
        if repository.exists(&lock_file) == Presence::Absent {
            return Ok(None);
        }
        let text = repository.read(&lock_file)?;
        self.manifest
            .lock(&DefinitionText(text.0))
            .map(Some)
            .map_err(|source| AddError::Manifest {
                file: lock_file.0.to_string(),
                source,
            })
    }

    /// The role of the `tastemaker.toml` at `path`, root or local, or why it does not read. Only a root file below the
    /// workspace makes a nested workspace whose tree it does not govern.
    fn role_of(
        &self,
        repository: &dyn crate::outbound::repository::interface::Repository,
        path: &RepoPath,
    ) -> Result<Role, AddError> {
        let text = repository.read(path)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(|source| AddError::Manifest {
                file: path.0.to_string(),
                source,
            })
    }

    /// `add` over the ports the edge injects.
    #[must_use]
    pub fn new(
        repository: Arc<dyn RepositoryPort>,
        manifest: Arc<dyn ManifestPort>,
        packages: Arc<dyn PackagesPort>,
        matching: Arc<dyn MatchingPort>,
    ) -> Self {
        Self {
            repository,
            manifest,
            packages,
            matching,
        }
    }

    /// Resolves the prospective manifest and every selected local file before either output is written.
    fn prospective_lock(
        &self,
        found: &dyn FoundRepository,
        file: &RepoPath,
        edited: &DefinitionText,
        from: &PackageRoot,
    ) -> Result<DefinitionText, AddError> {
        let repository = found;
        let manifest_error = |source| AddError::Manifest {
            file: file.0.to_string(),
            source,
        };
        let root = self
            .manifest
            .workspace_file(edited)
            .map_err(manifest_error)?;
        let mut workspace =
            unloaded(file, found.workspace(), root).ok_or_else(|| AddError::Resolution {
                message: MISSING_ROOT.into(),
            })?;
        apply_variables(
            &mut workspace.composition,
            &mut [
                &mut workspace.section.members,
                &mut workspace.section.rules_exclude,
            ],
        );
        // Each entry is at what the lock pins, the one added at the latest its package offers.
        let pins = self.pins(found)?;
        let compile = |spelling: &Spelling| self.matching.compile(spelling);
        // Every path of the root file is relative to its directory, the workspace's.
        let base = found.workspace().clone();
        let values = Interpolations::default();
        let context = SelectionContext {
            base: &base,
            values: &values,
            instances: &[],
            placed: &[],
            compile: &compile,
        };
        let selector_error = |error: SelectorsError| AddError::Resolution {
            message: error.to_string(),
        };
        let members =
            resolve_selectors(&workspace.section.members, &context).map_err(selector_error)?;
        let excluded = resolve_selectors(&workspace.section.rules_exclude, &context)
            .map_err(selector_error)?;
        let member = |directory: &RepoPath| {
            if first_match(&members, directory).is_some() {
                Coverage::Covered
            } else {
                Coverage::Uncovered
            }
        };
        workspace.files = governed(
            repository.list(&base)?.0,
            file,
            member,
            |path: &RepoPath| self.role_of(repository, path),
        )?;
        let local_paths = locals(&workspace.files, file, member, |path| {
            covers_any(&excluded, path)
        });
        workspace.locals = self.read_locals(found, &local_paths)?;
        let direct = workspace
            .section
            .dependencies
            .iter()
            .map(|dependency| dependency.package.clone())
            .collect::<Vec<_>>();
        self.load_packages(&mut workspace, from)?;
        // A version the package cannot list is a failure, said once the pins are made.
        let failed = std::cell::RefCell::new(None::<PackagesError>);
        let version_of = |release: &Release| {
            if let Some(pinned) = pins.as_ref().and_then(|lock| locked_version(lock, release)) {
                return pinned;
            }
            let Some(package) = workspace.packages.get(&release.package) else {
                return VersionText(Box::default());
            };
            let within =
                PackageFile(format!("{}/{}", kind_directory(release.kind), release.name.0).into());
            match self.packages.versions(&package.directory, &within) {
                Ok(offered) => latest(&VersionText(Box::default()), offered.iter()),
                Err(error) => {
                    if failed.borrow().is_none() {
                        *failed.borrow_mut() = Some(error);
                    }
                    VersionText(Box::default())
                }
            }
        };
        let mut composition = workspace.composition.clone();
        pin(&mut composition, &version_of);
        let mut pinned_locals = workspace.locals.clone();
        for each in &mut pinned_locals {
            pin(&mut each.composition, &version_of);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(error.into());
        }
        workspace.composition = composition;
        workspace.locals = pinned_locals;
        let loaded = self.load_definitions(workspace)?;
        let lock = lock(&direct, &loaded.packages, shared(&loaded).iter());
        Ok(self.manifest.lock_text(&lock))
    }

    /// Reads the selected member manifests before their definitions are resolved.
    fn read_locals(
        &self,
        found: &dyn FoundRepository,
        paths: &[&RepoPath],
    ) -> Result<Vec<LocalFile>, AddError> {
        let repository = found;
        let mut locals = Vec::new();
        for path in paths {
            let text = repository.read(path)?;
            let mut local_file = self
                .manifest
                .workspace_file(&DefinitionText(text.0))
                .map_err(|source| AddError::Manifest {
                    file: path.0.to_string(),
                    source,
                })?;
            if matches!(local_file.role, Role::Root(_)) {
                return Err(AddError::Resolution {
                    message: format!(
                        "`{}`: a local tastemaker.toml never has a workspace table",
                        path.0
                    ),
                });
            }
            apply_variables(&mut local_file.composition, &mut []);
            locals.push(local(path, local_file.composition));
        }
        Ok(locals)
    }

    /// Resolves direct and transitive dependencies before definition loading.
    fn load_packages(
        &self,
        workspace: &mut LoadedWorkspace,
        from: &PackageRoot,
    ) -> Result<(), AddError> {
        let mut queue = workspace
            .section
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.clone(),
                    from.clone(),
                    SourcePath(Box::default()),
                    Reach::Direct,
                )
            })
            .collect::<VecDeque<_>>();

        while let Some((dependency, origin, base, reach)) = queue.pop_front() {
            if workspace.packages.contains_key(&dependency.package) {
                continue;
            }
            let located = self.packages.locate(&dependency, &origin, &base)?;
            let text = self
                .packages
                .read(&located.directory, &PackageFile(PACKAGE_FILE.into()))?;
            let manifest = self
                .manifest
                .package(&DefinitionText(text.0))
                .map_err(|source| AddError::Manifest {
                    file: format!("{}/{PACKAGE_FILE}", located.directory.0),
                    source,
                })?;
            for own in &manifest.dependencies {
                queue.push_back((
                    own.clone(),
                    located.directory.clone(),
                    located.base.clone(),
                    Reach::Transitive,
                ));
            }
            drop(workspace.packages.insert(
                dependency.package,
                ResolvedPackage {
                    directory: located.directory,
                    pin: located.pin,
                    manifest,
                    reach,
                },
            ));
        }
        Ok(())
    }

    /// Reads the definitions requested by the pure loading state machine.
    fn load_definitions(&self, workspace: LoadedWorkspace) -> Result<LoadedWorkspace, AddError> {
        let mut loading = begin(workspace);
        while let Some(wanted) = next(&mut loading) {
            let text = self
                .packages
                .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))?;
            let text = DefinitionText(text.0);
            let parse_error = |source| AddError::Manifest {
                file: format!("{}/{}", wanted.directory.0, wanted.file.0),
                source,
            };
            let definition = self
                .manifest
                .definition(wanted.release.kind, &text)
                .map_err(parse_error)?;
            definition_loaded(&mut loading, &wanted, definition);
        }
        let loaded = finish(loading);
        if let Some(problem) = loaded.problems.first() {
            return Err(AddError::Resolution {
                message: problem.0.to_string(),
            });
        }
        Ok(loaded.workspace)
    }

    /// The description a definition file gives.
    fn description(
        &self,
        kind: DefinitionKind,
        text: &DefinitionText,
    ) -> Result<
        crate::domain::entries::types::Description,
        crate::outbound::manifest::errors::ManifestError,
    > {
        self.manifest
            .definition(kind, text)
            .map(|definition| description(&definition))
    }
}

impl AddUseCase for AddUseCaseImpl {
    fn add(&self, request: AddRequest) -> Result<AddReport, AddError> {
        let found = self.repository.find(&request.start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let manifest_error = |source| AddError::Manifest {
            file: file.0.to_string(),
            source,
        };
        let workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0.clone()))
            .map_err(manifest_error)?;
        let Role::Root(section) = workspace.role else {
            return Err(AddError::NoDefinition {
                definition: request.definition.0.to_string(),
            });
        };
        let (package_name, name) = match request.definition.0.rsplit_once('/') {
            Some((package, name)) => (Some(package), name),
            None => (None, &*request.definition.0),
        };
        let from = root_at(found.root(), found.workspace());
        for dependency in &section.dependencies {
            if package_name.is_some_and(|package| package != &*dependency.package.0) {
                continue;
            }
            let located = self
                .packages
                .locate(dependency, &from, &SourcePath(Box::default()))?;
            for kind in [
                DefinitionKind::Spec,
                DefinitionKind::Template,
                DefinitionKind::Rule,
            ] {
                let within = format!("{}/{name}", kind_directory(kind));
                let versions = self
                    .packages
                    .versions(&located.directory, &PackageFile(within.clone().into()))?;
                let Some(first) = versions.first() else {
                    continue;
                };
                let version = latest(first, versions.iter());
                let definition_text = self.packages.read(
                    &located.directory,
                    &PackageFile(format!("{within}/{}/{}", version.0, kind_file(kind)).into()),
                )?;
                let description = self
                    .description(kind, &DefinitionText(definition_text.0))
                    .map_err(manifest_error)?;
                let at = super::service::entry_path(found.as_ref(), PathText(&request.at.0));
                let pattern = PathPattern(at.0.clone());
                let draft = EntryDraft {
                    kind,
                    package: dependency.package.clone(),
                    version: version.clone(),
                    name: DefinitionName(name.into()),
                    include: Selectors(vec![Selector::Glob(pattern.clone())]),
                    file: pattern.clone(),
                    // The count the path says: one for a literal path, at least one for a glob or a capture, so the entry
                    // writes none.
                    count: derived(&[Some(&pattern)], &std::collections::BTreeSet::new()),
                    params: literal_params(&request.params),
                    description,
                };
                let edited = self
                    .manifest
                    .add_entry(&text, &draft)
                    .map_err(manifest_error)?;
                let lock = self.prospective_lock(
                    found.as_ref(),
                    &file,
                    &DefinitionText(edited.0.clone()),
                    &from,
                )?;
                let novelty = if edited.0 == text.0 {
                    Novelty::Held
                } else {
                    Novelty::New
                };
                if request.writing == Writing::Write {
                    repository.write(&file, &edited)?;
                    let lock_file =
                        join(found.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
                    repository
                        .write(&lock_file, &FileText(lock.0))
                        .map_err(|source| AddError::Relock { source })?;
                }
                return Ok(AddReport {
                    file,
                    entry: ShownEntry(
                        format!("{}/{name}@{}", dependency.package.0, version.0).into(),
                    ),
                    reference: DefinitionName(name.into()),
                    kind,
                    novelty,
                });
            }
        }
        Err(AddError::NoDefinition {
            definition: request.definition.0.to_string(),
        })
    }
}
