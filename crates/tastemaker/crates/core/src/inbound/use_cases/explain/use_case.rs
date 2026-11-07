// ignore: idiomatic.arch/file-length - explain is one flow over four ports: it finds the workspace, reads its files and packages, loads every definition its entries name, and writes the pages it answers with; each step reads or writes through a port and hands its result to the next, while what the steps decide (placement, planning, selection, the pages' text) is in the domain concepts it calls, so what is left is the order of the port calls, which no domain concept can hold.
//! The `explain` use case: the workspace read and placed as `check` places it, and what applies to one path said; or a
//! definition, a topic, or the workspace's scopes.

use super::service;
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use super::errors::ExplainError;
use super::interface::ExplainUseCase;
use super::types::{Asked, ExplainReport, ExplainRequest, NameAsked, Opened};
use crate::domain::applications::types::{Plan, Purpose};
use crate::domain::captures::types::Interpolations;
use crate::domain::catalog::service::{grammar_sections, operator_sections, relation_sections};
use crate::domain::catalog::types::{CatalogName, OperatorAnswer, RelationAnswer};
use crate::domain::definitions::constants::WORKSPACE_FILE;
use crate::domain::definitions::service::{local, locals, package_paths};
use crate::domain::definitions::types::{ErrorFile, Governed, LoadedWorkspace, LocalPackage, Role};
use crate::domain::entries::service::identity;
use crate::domain::explanations::service::{
    definition_explained, page, path_sections, placed_by, rule_line, scaffold_line, scope_line,
};
use crate::domain::explanations::types::{Section, SectionLine, SectionTitle};
use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::loading::service::{begin, definition_loaded, finish, next};
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::service::{kind_directory, kind_file, kind_of, latest};
use crate::domain::packages::types::{
    DefinitionKind, DefinitionName, KindWord, PackageName, PackageRoot, Reach, ResolvedPackage,
    SourcePath,
};
use crate::domain::paths::service::{containment, join, relative};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::{Containment, PathText};
use crate::domain::placement::service::place;
use crate::domain::placement::types::Placement;
use crate::domain::planning::service::plan;
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{Identity, Origin};
use crate::domain::selectors::service::{covers_any, first_match, resolve};
use crate::domain::selectors::types::{
    Binding, Coverage, PathTest, PatternFailure, SelectionContext, Spelling,
};
use crate::domain::topics::constants::{OPERATOR_WORD, RELATION_WORD, RYOQL};
use crate::domain::topics::service::{text, topic, topics};
use crate::domain::variables::service::apply as apply_variables;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::interface::FoundRepository;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;

/// `explain`, over the ports the edge injects.
pub struct ExplainUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
    /// Answers the analysis the definitions ask for.
    analysis: Arc<dyn AnalysisPort>,
}

impl std::fmt::Debug for ExplainUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ExplainUseCaseImpl {
    /// The role of the `tastemaker.toml` at `path`, root or local, or why it does not read. Only a root file below the
    /// workspace makes a nested workspace whose tree it does not govern.
    fn role_of(
        &self,
        repository: &dyn crate::outbound::repository::interface::Repository,
        path: &RepoPath,
    ) -> Result<Role, ExplainError> {
        let text = repository.read(path)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(|source| ExplainError::Manifest {
                file: path.0.to_string(),
                source,
            })
    }

    /// `explain` over the ports the edge injects.
    #[must_use]
    pub fn new(
        repository: Arc<dyn RepositoryPort>,
        manifest: Arc<dyn ManifestPort>,
        packages: Arc<dyn PackagesPort>,
        matching: Arc<dyn MatchingPort>,
        analysis: Arc<dyn AnalysisPort>,
    ) -> Self {
        Self {
            repository,
            manifest,
            packages,
            matching,
            analysis,
        }
    }

    /// Whether `name` is a path on disk from the start, which wins over a topic or a catalog page of the same name; no
    /// workspace found is no path.
    fn named_path(&self, start: &MachinePath, name: &NameAsked) -> Presence {
        self.repository
            .find(start)
            .map_or(Presence::Absent, |found| {
                let path = join(found.start(), PathText(&name.0)).unwrap_or_default();
                found.exists(&path)
            })
    }

    /// What is said of `name` without reading the workspace: a relation or operator page, the grammar, or a topic; `None`
    /// for any other name.
    fn unloaded(&self, name: &NameAsked) -> Result<Option<Vec<Section>>, ExplainError> {
        if let Some((kind, entry)) = name.0.split_once('/')
            && let Some(sections) = self.catalogued(KindWord(kind), &CatalogName(entry.into()))?
        {
            return Ok(Some(sections));
        }
        if &*name.0 == RYOQL {
            return Ok(Some(grammar_sections(&self.analysis.grammar()?.0)));
        }
        Ok(
            topic(crate::domain::topics::types::TopicName(&name.0)).map(|found| {
                vec![Section {
                    title: SectionTitle(found.name.0.into()),
                    lines: vec![SectionLine(text(&found).0.into())],
                }]
            }),
        )
    }

    /// What ryo's catalog says of `<kind>/<name>`, for a relation or an operator; `None` for another kind or a name the
    /// catalog has not.
    fn catalogued(
        &self,
        kind: KindWord<'_>,
        name: &CatalogName,
    ) -> Result<Option<Vec<Section>>, ExplainError> {
        Ok(match kind.0 {
            RELATION_WORD => match self.analysis.relation(name)? {
                RelationAnswer::Found(page) => Some(relation_sections(&page)),
                RelationAnswer::Unknown => None,
            },
            OPERATOR_WORD => match self.analysis.operator(name)? {
                OperatorAnswer::Found(page) => Some(operator_sections(&page)),
                OperatorAnswer::Unknown => None,
            },
            _ => None,
        })
    }

    /// The spellings' compiler, over the matching port.
    fn compile(
        &self,
    ) -> impl Fn(&Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> + Sync + '_ {
        |spelling| self.matching.compile(spelling)
    }

    /// The compiler of the glob sets a plan matches with, over the matching port.
    fn sets(
        &self,
    ) -> impl Fn(
        &[crate::domain::selectors::types::SetGlob],
    ) -> Result<Arc<dyn crate::domain::selectors::types::PathSet>, PatternFailure>
    + Sync
    + '_ {
        |globs| self.matching.compile_set(globs)
    }

    /// The files the workspace governs, every file beneath its directory but a nested workspace's, and its local
    /// `tastemaker.toml` files read.
    fn governed(
        &self,
        found: &dyn FoundRepository,
        section: &crate::domain::definitions::types::WorkspaceSection,
        file: &RepoPath,
    ) -> Result<Governed, ExplainError> {
        let repository = found;
        let compile = self.compile();
        let none = Interpolations::default();
        // Every path of the root file is relative to its directory, the workspace's.
        let root = found.workspace().clone();
        let context = SelectionContext {
            base: &root,
            values: &none,
            instances: &[],
            placed: &[],
            compile: &compile,
        };
        let selector = |source| {
            ExplainError::Placement(crate::domain::placement::errors::PlacementError::Selector {
                entry: crate::domain::template_placing::constants::WORKSPACE_TABLE.to_owned(),
                source,
            })
        };
        let members = resolve(&section.members, &context).map_err(selector)?;
        let excluded = resolve(&section.rules_exclude, &context).map_err(selector)?;
        let member = |directory: &RepoPath| {
            if first_match(&members, directory).is_some() {
                Coverage::Covered
            } else {
                Coverage::Uncovered
            }
        };
        let files = crate::domain::definitions::service::governed(
            repository.list(&root)?.0,
            file,
            member,
            |path: &RepoPath| self.role_of(repository, path),
        )?;
        let mut local_files = Vec::new();
        for path in locals(&files, file, member, |path| covers_any(&excluded, path)) {
            let text = repository.read(path)?;
            let mut read = self
                .manifest
                .workspace_file(&DefinitionText(text.0))
                .map_err(service::manifest_error(ErrorFile(&path.0)))?;
            apply_variables(&mut read.composition, &mut []);
            local_files.push(local(path, read.composition));
        }
        Ok(Governed {
            files,
            locals: local_files,
        })
    }

    /// The workspace's packages, direct and transitive, breadth first.
    fn resolved(
        &self,
        found: &dyn FoundRepository,
        section: &crate::domain::definitions::types::WorkspaceSection,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, ExplainError> {
        let project = PackageRoot(
            std::path::Path::new(&*found.root().0)
                .join(&*found.workspace().0)
                .display()
                .to_string()
                .into(),
        );
        let mut queue = section
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.clone(),
                    project.clone(),
                    SourcePath(Box::default()),
                    Reach::Direct,
                )
            })
            .collect::<VecDeque<_>>();
        let mut packages: BTreeMap<PackageName, ResolvedPackage> = BTreeMap::new();
        while let Some((dependency, from, base, reach)) = queue.pop_front() {
            if packages.contains_key(&dependency.package) {
                continue;
            }
            let located = self.packages.locate(&dependency, &from, &base)?;
            let text = self
                .packages
                .read(&located.directory, &PackageFile(PACKAGE_FILE.into()))?;
            let manifest = self
                .manifest
                .package(&DefinitionText(text.0))
                .map_err(service::manifest_error(ErrorFile(PACKAGE_FILE)))?;
            for own in &manifest.dependencies {
                queue.push_back((
                    own.clone(),
                    located.directory.clone(),
                    located.base.clone(),
                    Reach::Transitive,
                ));
            }
            drop(packages.insert(
                dependency.package.clone(),
                ResolvedPackage {
                    directory: located.directory,
                    pin: located.pin,
                    manifest,
                    reach,
                },
            ));
        }
        Ok(packages)
    }

    /// Reads every definition the load asks for, until it asks for none.
    fn definitions(
        &self,
        loading: &mut crate::domain::loading::types::Loading,
    ) -> Result<(), ExplainError> {
        while let Some(wanted) = next(loading) {
            let text = DefinitionText(
                self.packages
                    .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))?
                    .0,
            );
            let definition = self
                .manifest
                .definition(wanted.release.kind, &text)
                .map_err(service::manifest_error(ErrorFile(&wanted.file.0)))?;
            definition_loaded(loading, &wanted, definition);
        }
        Ok(())
    }

    /// Writes every topic and every definition the workspace loads, one page each, and an index, into `directory`.
    fn everything(
        &self,
        workspace: &LoadedWorkspace,
        directory: &MachinePath,
    ) -> Result<Vec<RepoPath>, ExplainError> {
        let directory = self.repository.open(directory);
        let mut written = Vec::new();
        let mut index: Vec<String> = Vec::new();
        for found in topics() {
            let path = RepoPath(format!("topics/{}.md", found.name.0).into());
            directory.write(&path, &FileText(text(found).0.into()))?;
            index.push(format!("- [{}]({})\n", found.name.0, path.0));
            written.push(path);
        }
        let grammar_path = RepoPath(format!("topics/{RYOQL}.md").into());
        let grammar = page(&grammar_sections(&self.analysis.grammar()?.0)).0;
        directory.write(&grammar_path, &FileText(grammar))?;
        index.push(format!("- [{RYOQL}]({})\n", grammar_path.0));
        written.push(grammar_path);
        for (kind, entries) in [
            (RELATION_WORD, self.analysis.relations()?.0),
            (OPERATOR_WORD, self.analysis.operators()?.0),
        ] {
            for entry in entries {
                let Some(sections) = self.catalogued(KindWord(kind), &entry.name)? else {
                    continue;
                };
                let path = RepoPath(format!("{kind}s/{}.md", entry.name.0).into());
                directory.write(&path, &FileText(page(&sections).0))?;
                index.push(format!("- [{kind}/{}]({})\n", entry.name.0, path.0));
                written.push(path);
            }
        }
        for release in workspace
            .rules
            .keys()
            .chain(workspace.specs.keys())
            .chain(workspace.templates.keys())
        {
            let Some(sections) = self.definition(workspace, release.kind, &release.name)? else {
                continue;
            };
            let text = page(&sections).0;
            let path =
                RepoPath(format!("{}/{}.md", kind_directory(release.kind), release.name.0).into());
            directory.write(&path, &FileText(text))?;
            index.push(format!("- [{}]({})\n", release.name.0, path.0));
            written.push(path);
        }
        let path = RepoPath(crate::domain::topics::constants::INDEX.into());
        directory.write(&path, &FileText(index.concat().into()))?;
        written.push(path);
        written.sort();
        written.dedup();
        Ok(written)
    }

    /// Pins each entry of the workspace's `composition` and of its `locals` at what the lock pins, else at the latest
    /// version its package offers.
    fn pin(
        &self,
        found: &dyn FoundRepository,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        composition: &mut crate::domain::entries::types::Composition,
        locals: &mut [crate::domain::definitions::types::LocalFile],
    ) -> Result<(), ExplainError> {
        let repository = found;
        let lock_file = join(
            found.workspace(),
            PathText(crate::domain::definitions::constants::LOCK_FILE),
        )
        .unwrap_or_default();
        // A lock that does not read is an error to say, never one to rewrite as if it were missing.
        let pins = if repository.exists(&lock_file) == Presence::Absent {
            None
        } else {
            let lock_text = repository.read(&lock_file)?;
            Some(
                self.manifest
                    .lock(&DefinitionText(lock_text.0))
                    .map_err(service::manifest_error(ErrorFile(&lock_file.0)))?,
            )
        };
        // A version the package cannot list is a failure, said once the pins are made.
        let failed =
            std::cell::RefCell::new(None::<crate::outbound::packages::errors::PackagesError>);
        let version_of = |release: &crate::domain::packages::types::Release| {
            if let Some(pinned) = pins
                .as_ref()
                .and_then(|lock| crate::domain::packages::service::locked_version(lock, release))
            {
                return pinned;
            }
            packages.get(&release.package).map_or_else(
                || crate::domain::packages::types::VersionText(Box::default()),
                |package| {
                    let offered = match self.packages.versions(
                        &package.directory,
                        &PackageFile(
                            format!("{}/{}", kind_directory(release.kind), release.name.0).into(),
                        ),
                    ) {
                        Ok(offered) => offered,
                        Err(error) => {
                            if failed.borrow().is_none() {
                                *failed.borrow_mut() = Some(error);
                            }
                            return crate::domain::packages::types::VersionText(Box::default());
                        }
                    };
                    latest(
                        &crate::domain::packages::types::VersionText(Box::default()),
                        offered.iter(),
                    )
                },
            )
        };
        crate::domain::entries::service::pin(composition, &version_of);
        for each in locals {
            crate::domain::entries::service::pin(&mut each.composition, &version_of);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(error.into());
        }
        Ok(())
    }

    /// The workspace nearest `start`, read: its files, its packages and every definition its entries name.
    fn load(&self, start: &MachinePath) -> Result<Opened, ExplainError> {
        let found = self.repository.find(start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let mut workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0))
            .map_err(service::manifest_error(ErrorFile(&file.0)))?;
        let Role::Root(mut section) = workspace.role.clone() else {
            return Err(ExplainError::Unknown {
                name: file.0.to_string(),
            });
        };
        apply_variables(
            &mut workspace.composition,
            &mut [&mut section.rules_exclude, &mut section.members],
        );
        let Governed {
            files,
            locals: mut local_files,
        } = self.governed(found.as_ref(), &section, &file)?;
        let packages = self.resolved(found.as_ref(), &section)?;
        self.pin(
            found.as_ref(),
            &packages,
            &mut workspace.composition,
            &mut local_files,
        )?;
        let local_packages = package_paths(
            &packages,
            crate::domain::definitions::types::RepositoryText(&found.root().0),
        )
        .into_iter()
        .filter(|local| {
            files
                .iter()
                .any(|candidate| containment(candidate, &local.path) == Containment::Inside)
        })
        .map(|local| LocalPackage {
            package: local.name,
            directory: local.path,
        })
        .collect::<Vec<_>>();
        let governed = files
            .into_iter()
            .filter(|candidate| {
                !local_packages.iter().any(|package| {
                    containment(candidate, &package.directory) == Containment::Inside
                })
            })
            .collect();
        let mut loading = begin(LoadedWorkspace {
            file,
            directory: found.workspace().clone(),
            section,
            composition: workspace.composition,
            locals: local_files,
            packages,
            specs: BTreeMap::new(),
            rules: BTreeMap::new(),
            templates: BTreeMap::new(),
            latest: BTreeMap::new(),
            files: governed,
            local_packages,
        });
        self.definitions(&mut loading)?;
        let crate::domain::loading::types::WorkspaceLoad {
            workspace: loaded, ..
        } = finish(loading);
        Ok(Opened {
            found,
            workspace: loaded,
        })
    }

    /// What applies to `path`: the scopes it lies in, the entry that places it, and the rules that read it.
    /// What applies to `path`: its scopes, what places it and the rules that read it. Each spec instance it lies in, and
    /// the template entry that places it, comes with the `scaffold` command that writes what that entry places, its
    /// `--at` relative to `start`, where `explain` ran; none when that directory is not below `start`.
    fn path(
        &self,
        workspace: &LoadedWorkspace,
        placement: &Placement,
        plan: &Plan,
        path: &RepoPath,
        start: &RepoPath,
    ) -> Vec<Section> {
        let mut scopes = Vec::new();
        for scope in placement.scopes.iter().filter(|scope| {
            containment(path, &scope.dir) == Containment::Inside
                && !scope
                    .nested
                    .iter()
                    .any(|nested| containment(path, nested) == Containment::Inside)
        }) {
            scopes.push(scope_line(scope));
            if let Origin::Instance { .. } = scope.origin
                && let Some(at) = relative(&scope.dir, start)
            {
                scopes.push(scaffold_line(&scope.identity, at));
            }
        }
        let template_at = placement.placed.get(path).and_then(|file| {
            let scope = placement.scopes.get(file.scope.0)?;
            let entry = composition_of(workspace, &scope.composition)
                .templates
                .get(file.entry.0)?;
            Some((
                Identity(identity(&entry.header, DefinitionKind::Template, &entry.template).into()),
                relative(&scope.dir, start),
            ))
        });
        let placer = template_at.as_ref().map(|(identity, _)| identity.clone());
        let mut rules = Vec::new();
        if let Some(rest) = relative(path, &plan.base).map(|relative| relative.0) {
            for rule in &plan.rules {
                let Some(Purpose::Rule(rendering)) = plan.purposes.get(&rule.id) else {
                    continue;
                };
                let reads = rule.include.iter().any(|glob| {
                    self.matching
                        .compile(&Spelling {
                            glob: crate::domain::selectors::types::SpelledGlob(glob.0.clone()),
                            binder: Binding::GlobOnly,
                            groups: Vec::new(),
                        })
                        .is_ok_and(|test| {
                            test.passes(crate::domain::selectors::types::PathRest(rest))
                                == Coverage::Covered
                        })
                });
                if reads {
                    rules.push(rule_line(&rendering.rule));
                }
            }
        }
        let mut placing = placed_by(placer.as_ref());
        if let Some((identity, Some(at))) = &template_at {
            placing.push(scaffold_line(identity, *at));
        }
        path_sections(scopes, placing, rules)
    }

    /// What a definition is, at its latest version in the workspace's packages. The name may be qualified as this
    /// answer's own title writes it, `<package>/<name>` and `<package>/<name>@<version>`, so a title can be asked again:
    /// a package names the one package looked in, and a version the one version read.
    fn definition(
        &self,
        workspace: &LoadedWorkspace,
        kind: DefinitionKind,
        name: &DefinitionName,
    ) -> Result<Option<Vec<Section>>, ExplainError> {
        let (wanted, rest) = match name.0.rsplit_once('/') {
            Some((package, rest)) => (Some(package), rest),
            None => (None, &*name.0),
        };
        let (bare, pinned) = match rest.split_once('@') {
            Some((bare, version)) => (bare, Some(version)),
            None => (rest, None),
        };
        for (package, resolved) in &workspace.packages {
            if wanted.is_some_and(|wanted| wanted != &*package.0) {
                continue;
            }
            let within = format!("{}/{}", kind_directory(kind), bare);
            let versions = self
                .packages
                .versions(&resolved.directory, &PackageFile(within.clone().into()))?;
            let Some(first) = versions.first() else {
                continue;
            };
            let version = match pinned {
                Some(pinned) => match versions.iter().find(|version| &*version.0 == pinned) {
                    Some(version) => version.clone(),
                    None => return Ok(None),
                },
                None => latest(first, versions.iter()),
            };
            let file = format!("{within}/{}/{}", version.0, kind_file(kind));
            let text = DefinitionText(
                self.packages
                    .read(&resolved.directory, &PackageFile(file.clone().into()))?
                    .0,
            );
            let title = SectionTitle(format!("{}/{}@{}", package.0, bare, version.0).into());
            let definition = self
                .manifest
                .definition(kind, &text)
                .map_err(service::manifest_error(ErrorFile(&file)))?;
            return Ok(Some(definition_explained(title, &definition)));
        }
        Ok(None)
    }
}

impl ExplainUseCase for ExplainUseCaseImpl {
    fn explain(&self, request: ExplainRequest) -> Result<ExplainReport, ExplainError> {
        if let Asked::Named(name) = &request.asked
            && self.named_path(&request.start, name) == Presence::Absent
            && let Some(sections) = self.unloaded(name)?
        {
            return Ok(ExplainReport {
                sections,
                written: Vec::new(),
            });
        }
        let Opened { found, workspace } = self.load(&request.start)?;
        let repository = &*found;
        let compile = self.compile();
        let placement = place(&workspace, &compile, &*self.manifest)?;
        let base = workspace.directory.clone();
        let planned = plan(
            &workspace,
            &placement,
            base,
            crate::domain::applications::types::Compilers {
                compile: &compile,
                sets: &self.sets(),
            },
        )?;
        match request.asked {
            Asked::Workspace => Ok(ExplainReport {
                sections: service::overview(
                    &workspace,
                    &placement,
                    &planned,
                    &self.repository.skipped().0,
                ),
                written: Vec::new(),
            }),
            Asked::Named(name) => {
                let path = join(found.start(), PathText(&name.0)).unwrap_or_default();
                if repository.exists(&path) == Presence::Present {
                    return Ok(ExplainReport {
                        sections: self.path(&workspace, &placement, &planned, &path, found.start()),
                        written: Vec::new(),
                    });
                }
                if let Some((kind, definition)) = name.0.split_once('/')
                    && let Some(kind) = kind_of(KindWord(kind))
                    && let Some(sections) =
                        self.definition(&workspace, kind, &DefinitionName(definition.into()))?
                {
                    return Ok(ExplainReport {
                        sections,
                        written: Vec::new(),
                    });
                }
                match self.unloaded(&name)? {
                    Some(sections) => Ok(ExplainReport {
                        sections,
                        written: Vec::new(),
                    }),
                    None => Err(ExplainError::Unknown {
                        name: name.0.to_string(),
                    }),
                }
            }
            Asked::Everything(directory) => Ok(ExplainReport {
                sections: Vec::new(),
                written: self.everything(&workspace, &directory)?,
            }),
        }
    }
}
