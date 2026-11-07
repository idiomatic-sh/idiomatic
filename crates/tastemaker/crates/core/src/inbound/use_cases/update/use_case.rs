// ignore: idiomatic.arch/file-length - `update` is one use case: read each entry's latest version, select the moves, relock and verify the prospective lock, each step an orchestration over the ports the use case holds.
//! The `update` use case: each entry's latest version read from its package, and the lock, which pins each entry's
//! version, moved to it.

use super::errors::UpdateError;
use super::interface::UpdateUseCase;
use super::types::{Entries, Move, UpdateReport, UpdateRequest, VersionChoice};
use crate::domain::captures::types::Interpolations;
use crate::domain::definitions::constants::{LOCK_FILE, MISSING_ROOT, WORKSPACE_FILE};
use crate::domain::definitions::service::{governed, local, locals, shared, unloaded};
use crate::domain::definitions::types::{LoadedWorkspace, LocalFile, Role};
use crate::domain::entries::service::named_releases;
use crate::domain::entries::service::pin;
use crate::domain::entries::types::NamedRelease;
use crate::domain::files::types::FileText;
use crate::domain::loading::service::{begin, definition_loaded, finish, next};
use crate::domain::packages::constants::{CHANGELOG_FILE, PACKAGE_FILE};
use crate::domain::packages::service::{
    changes, kind_directory, kind_file, latest, lock, locked_version, offered_version, root_at,
    version_number,
};
use crate::domain::packages::types::{
    ChangeLog, Lock, PackageRoot, Reach, Release, ResolvedPackage, SourcePath, VersionText,
};
use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::domain::selectors::errors::SelectorsError;
use crate::domain::selectors::service::resolve as resolve_selectors;
use crate::domain::selectors::service::{covers_any, first_match};
use crate::domain::selectors::types::{Coverage, SelectionContext, Spelling};
use crate::domain::variables::service::apply as apply_variables;
use crate::domain::writing::types::Writing;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::interface::FoundRepository;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;
use std::collections::VecDeque;
use std::sync::Arc;

/// `update`, over the ports the edge injects.
pub struct UpdateUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
}

impl std::fmt::Debug for UpdateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl UpdateUseCaseImpl {
    /// What the workspace's lock pins; none when there is no lock. A lock that does not read is an error to say, never a
    /// lock to treat as missing and rewrite.
    fn pins(&self, found: &dyn FoundRepository) -> Result<Option<Lock>, UpdateError> {
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
            .map_err(|source| UpdateError::Manifest {
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
    ) -> Result<Role, UpdateError> {
        let text = repository.read(path)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(|source| UpdateError::Manifest {
                file: path.0.to_string(),
                source,
            })
    }

    /// `update` over the ports the edge injects.
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

    /// Reads the optional changelog for the release interval being crossed.
    fn read_changes(
        &self,
        directory: &PackageRoot,
        within: &PackageFile,
        before: &VersionText,
        after: &VersionText,
    ) -> Result<ChangeLog, UpdateError> {
        let notes_version = if version_number(before) > version_number(after) {
            before
        } else {
            after
        };
        let version_directory = PackageFile(format!("{}/{}", within.0, notes_version.0).into());
        let files = self.packages.files(directory, &version_directory)?;
        let notes = if files.iter().any(|file| &*file.0 == CHANGELOG_FILE) {
            self.packages
                .read(
                    directory,
                    &PackageFile(format!("{}/{}", version_directory.0, CHANGELOG_FILE).into()),
                )?
                .0
        } else {
            Box::default()
        };
        Ok(changes(&ChangeLog(notes), before, after))
    }

    /// The lock the workspace makes with each entry at the version `version_of` answers, its root file and every
    /// selected local file resolved, before it is written.
    fn prospective_lock(
        &self,
        found: &dyn FoundRepository,
        file: &RepoPath,
        root: crate::domain::definitions::types::WorkspaceFile,
        from: &PackageRoot,
        version_of: &dyn Fn(&Release) -> Option<VersionText>,
    ) -> Result<DefinitionText, UpdateError> {
        let repository = found;
        let mut workspace =
            unloaded(file, found.workspace(), root).ok_or_else(|| UpdateError::Resolution {
                message: MISSING_ROOT.into(),
            })?;
        apply_variables(
            &mut workspace.composition,
            &mut [
                &mut workspace.section.members,
                &mut workspace.section.rules_exclude,
            ],
        );
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
        let selector_error = |error: SelectorsError| UpdateError::Resolution {
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
        // What `version_of` does not answer is at the latest its package offers.
        let packages = workspace.packages.clone();
        // A version the package cannot list is a failure, said once the pins are made.
        let failed = std::cell::RefCell::new(None::<PackagesError>);
        let pinned = |release: &Release| {
            version_of(release).unwrap_or_else(|| {
                let within = PackageFile(
                    format!("{}/{}", kind_directory(release.kind), release.name.0).into(),
                );
                packages.get(&release.package).map_or_else(
                    || VersionText(Box::default()),
                    |package| {
                        let offered = match self.packages.versions(&package.directory, &within) {
                            Ok(offered) => offered,
                            Err(error) => {
                                if failed.borrow().is_none() {
                                    *failed.borrow_mut() = Some(error);
                                }
                                return VersionText(Box::default());
                            }
                        };
                        latest(&VersionText(Box::default()), offered.iter())
                    },
                )
            })
        };
        pin(&mut workspace.composition, &pinned);
        for each in &mut workspace.locals {
            pin(&mut each.composition, &pinned);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(error.into());
        }
        let loaded = self.load_definitions(workspace)?;
        let lock = lock(&direct, &loaded.packages, shared(&loaded).iter());
        Ok(self.manifest.lock_text(&lock))
    }

    /// Reads the selected member manifests before their definitions are resolved.
    fn read_locals(
        &self,
        found: &dyn FoundRepository,
        paths: &[&RepoPath],
    ) -> Result<Vec<LocalFile>, UpdateError> {
        let repository = found;
        let mut locals = Vec::new();
        for path in paths {
            let text = repository.read(path)?;
            let mut local_file = self
                .manifest
                .workspace_file(&DefinitionText(text.0))
                .map_err(|source| UpdateError::Manifest {
                    file: path.0.to_string(),
                    source,
                })?;
            if matches!(local_file.role, Role::Root(_)) {
                return Err(UpdateError::Resolution {
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
    ) -> Result<(), UpdateError> {
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
                .map_err(|source| UpdateError::Manifest {
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

    /// The lock the workspace makes with each entry at the version `targets` gives it, or at the one `pins` holds.
    fn relocked(
        &self,
        found: &dyn FoundRepository,
        file: &RepoPath,
        workspace: &crate::domain::definitions::types::WorkspaceFile,
        from: &PackageRoot,
        pins: Option<&Lock>,
        targets: &[Release],
    ) -> Result<DefinitionText, UpdateError> {
        let version_of = |release: &Release| {
            targets
                .iter()
                .find(|to| {
                    to.package == release.package
                        && to.kind == release.kind
                        && to.name == release.name
                })
                .map(|to| to.version.clone())
                .or_else(|| {
                    let lock = pins?;
                    locked_version(lock, release)
                })
        };
        self.prospective_lock(found, file, workspace.clone(), from, &version_of)
    }

    /// Reads the definitions requested by the pure loading state machine.
    fn load_definitions(&self, workspace: LoadedWorkspace) -> Result<LoadedWorkspace, UpdateError> {
        let mut loading = begin(workspace);
        while let Some(wanted) = next(&mut loading) {
            let text = self
                .packages
                .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))?;
            let text = DefinitionText(text.0);
            let parse_error = |source| UpdateError::Manifest {
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
            return Err(UpdateError::Resolution {
                message: problem.0.to_string(),
            });
        }
        Ok(loaded.workspace)
    }
}

impl UpdateUseCase for UpdateUseCaseImpl {
    fn update(&self, request: UpdateRequest) -> Result<UpdateReport, UpdateError> {
        let found = self.repository.find(&request.start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0.clone()))
            .map_err(|source| UpdateError::Manifest {
                file: file.0.to_string(),
                source,
            })?;
        let Role::Root(section) = &workspace.role else {
            return Err(UpdateError::NoEntry {
                entry: file.0.to_string(),
            });
        };
        let from = root_at(found.root(), found.workspace());
        let lock_file = join(found.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
        let pins = self.pins(found.as_ref())?;
        let name = match &request.entries {
            Entries::Named(name) => Some(name),
            Entries::Every => None,
        };
        let mut moves = Vec::new();
        let mut targets: Vec<Release> = Vec::new();
        for NamedRelease { entry, release } in named_releases(&workspace.composition, name) {
            let Some(dependency) = section
                .dependencies
                .iter()
                .find(|dependency| dependency.package == release.package)
            else {
                continue;
            };
            let located = self
                .packages
                .locate(dependency, &from, &SourcePath(Box::default()))?;
            let within =
                PackageFile(format!("{}/{}", kind_directory(release.kind), release.name.0).into());
            let offered = self.packages.versions(&located.directory, &within)?;
            let current = pins
                .as_ref()
                .and_then(|lock| locked_version(lock, &release))
                .unwrap_or_else(|| VersionText(Box::default()));
            let requested = match &request.target {
                VersionChoice::Version(version) => Some(version),
                VersionChoice::Latest => None,
            };
            let to = offered_version(&current, requested, &offered).ok_or_else(|| {
                UpdateError::NoVersion {
                    entry: entry.0.to_string(),
                    version: requested.map_or_else(String::new, |version| version.0.to_string()),
                }
            })?;
            // A directory alone is not an offered asset: the definition must be there before the lock is written.
            self.packages.offers(
                &located.directory,
                &PackageFile(format!("{}/{}/{}", within.0, to.0, kind_file(release.kind)).into()),
            )?;
            if version_number(&to) == version_number(&current) {
                continue;
            }
            let changes = self.read_changes(&located.directory, &within, &current, &to)?;
            targets.push(Release {
                version: to.clone(),
                ..release
            });
            moves.push(Move {
                entry,
                from: current,
                to,
                changes,
            });
        }
        if let Some(name) = name
            && named_releases(&workspace.composition, Some(name)).is_empty()
        {
            return Err(UpdateError::NoEntry {
                entry: name.0.to_string(),
            });
        }
        let lock = self.relocked(
            found.as_ref(),
            &file,
            &workspace,
            &from,
            pins.as_ref(),
            &targets,
        )?;
        if request.writing == Writing::Write {
            repository
                .write(&lock_file, &FileText(lock.0))
                .map_err(|source| UpdateError::Relock { source })?;
        }
        Ok(UpdateReport { file, moves })
    }
}
