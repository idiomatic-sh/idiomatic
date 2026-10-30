// ignore: idiomatic.arch/file-length - scaffold coordinates six ports in one transaction: it loads the workspace and packages, stages missing files without overwriting authored files, asks the existing placement and planning services for each prospective tree, validates condition parameters through the applications service, asks the analysis and template ports until that tree settles, and only then publishes through the repository port. The staging records, repeated-state detection and write order belong to this transaction; captures, optionality, template context and placement decisions remain in their existing domain services. Splitting this transaction across modules would scatter its no-partial-write boundary rather than separate a domain concept.
//! The `scaffold` use case: the workspace read, the entry found, and each file it places that does not exist rendered from
//! its template and written. It never overwrites.

use super::service;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use super::errors::ScaffoldError;
use super::interface::ScaffoldUseCase;
use super::types::{ScaffoldReport, ScaffoldRequest, Workspace};
use crate::domain::applications::types::{Compilers, Purpose};
use crate::domain::captures::service::unescaped;
use crate::domain::captures::types::Template;
use crate::domain::captures::types::{CaptureName, CaptureValue, Captures, Interpolations};
use crate::domain::definitions::constants::{LOCK_FILE, WORKSPACE_FILE};
use crate::domain::definitions::service::{content, local, locals, package_paths};
use crate::domain::definitions::types::{Governed, LoadedWorkspace, RepositoryText, Role};
use crate::domain::entries::service::{reference, spec_release, template_release};
use crate::domain::entries::types::{Amount, Composition, Count, SpecEntry, TemplateEntry};
use crate::domain::files::types::FileText;
use crate::domain::files::types::MachinePath;
use crate::domain::loading::service::{begin, definition_loaded, finish, next};
use crate::domain::loading::types::{Loading, WorkspaceLoad};
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::types::{
    DefinitionKind, PackageName, PackageRoot, Reach, Release, ResolvedPackage, SourcePath,
};
use crate::domain::paths::service::join;
use crate::domain::paths::types::{PathText, RepoPath};
use crate::domain::placement::types::Placement;
use crate::domain::scaffolding::service::{
    condition_reads, declaring, fewest, first_render, first_writing, given, literal_values,
    local_split, named_by_directory, naming, optionality, placed_name, spec_reads, template_reads,
    unfilled, waited, waiting,
};
use crate::domain::scaffolding::types::{
    Declaring, Draft, FirstWriting, LocalSplit, MissingParam, Naming, Optionality, Placing, Reads,
    Staging, Waiting,
};
use crate::domain::scopes::service::composition_of;
use crate::domain::scopes::types::{ConditionAt, Origin, ScopeIndex};
use crate::domain::selectors::service::{covers_any, first_match, resolve};
use crate::domain::selectors::types::{
    Coverage, PathTest, PatternFailure, SelectionContext, Spelling,
};
use crate::domain::template_placing::constants::WORKSPACE_TABLE;
use crate::domain::variables::service::apply as apply_variables;
use crate::domain::writing::types::Writing;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{AnalysisRequest, Corpus, GivenFile, Hosted};
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::interface::FoundRepository;
use crate::outbound::repository::interface::RepositoryPort;
use crate::outbound::repository::types::Presence;
use crate::outbound::templates::interface::TemplatesPort;

/// `scaffold`, over the ports the edge injects.
pub struct ScaffoldUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
    /// Renders the templates' contents.
    templates: Arc<dyn TemplatesPort>,
    /// Answers the analysis the definitions ask for.
    analysis: Arc<dyn AnalysisPort>,
}

impl std::fmt::Debug for ScaffoldUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ScaffoldUseCaseImpl {
    /// What the workspace's lock pins; none when there is no lock. A lock that does not read is an error to say, never a
    /// lock to treat as missing and rewrite.
    fn pins(
        &self,
        found: &dyn FoundRepository,
    ) -> Result<Option<crate::domain::packages::types::Lock>, ScaffoldError> {
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
            .map_err(|source| ScaffoldError::Manifest {
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
    ) -> Result<Role, ScaffoldError> {
        let text = repository.read(path)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(|source| ScaffoldError::Manifest {
                file: path.0.to_string(),
                source,
            })
    }

    /// `scaffold` over the ports the edge injects.
    #[must_use]
    pub fn new(
        repository: Arc<dyn RepositoryPort>,
        manifest: Arc<dyn ManifestPort>,
        packages: Arc<dyn PackagesPort>,
        matching: Arc<dyn MatchingPort>,
        templates: Arc<dyn TemplatesPort>,
        analysis: Arc<dyn AnalysisPort>,
    ) -> Self {
        Self {
            repository,
            manifest,
            packages,
            matching,
            templates,
            analysis,
        }
    }

    /// The spellings' compiler, over the matching port.
    fn compile(
        &self,
    ) -> impl Fn(&Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> + Sync + '_ {
        |spelling| self.matching.compile(spelling)
    }

    /// What `at` captures under the first of a spec entry's `include` it matches, resolved from the directory of the
    /// file that holds the entry: `languages/{lang}` at `languages/demo` captures `lang = demo`, which the entry's params
    /// then read.
    ///
    /// # Errors
    ///
    /// `OutsideTarget` when `at` is not a directory the entry's `include` selects and its `exclude` leaves.
    fn captures(
        &self,
        entry: &SpecEntry,
        base: &RepoPath,
        at: &RepoPath,
    ) -> Result<Captures, ScaffoldError> {
        let compile = self.compile();
        let none = Interpolations::default();
        let context = SelectionContext {
            base,
            values: &none,
            instances: &[],
            placed: &[],
            compile: &compile,
        };
        let selector = |source| {
            ScaffoldError::Placement(crate::domain::placement::errors::PlacementError::Selector {
                entry: WORKSPACE_TABLE.to_owned(),
                source,
            })
        };
        let include = resolve(&entry.header.include, &context).map_err(selector)?;
        let exclude = resolve(&entry.header.exclude, &context).map_err(selector)?;
        match first_match(&include, at) {
            Some(found) if covers_any(&exclude, at) == Coverage::Uncovered => Ok(found.captures),
            _ => Err(ScaffoldError::OutsideTarget {
                at: at.0.to_string(),
                entry: reference(&entry.header, &entry.spec).0.to_string(),
            }),
        }
    }

    /// The files the workspace governs, every file beneath its directory but a nested workspace's, and its local
    /// `tastemaker.toml` files read.
    fn governed(
        &self,
        found: &dyn FoundRepository,
        section: &crate::domain::definitions::types::WorkspaceSection,
        file: &RepoPath,
    ) -> Result<Governed, ScaffoldError> {
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
            ScaffoldError::Placement(crate::domain::placement::errors::PlacementError::Selector {
                entry: WORKSPACE_TABLE.to_owned(),
                source,
            })
        };
        let members = resolve(&section.members, &context).map_err(selector)?;
        let excluded = resolve(&section.rules_exclude, &context).map_err(selector)?;
        let member =
            |directory: &RepoPath| Coverage::from(first_match(&members, directory).is_some());
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
                .map_err(service::manifest_error(PathText(&path.0)))?;
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
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, ScaffoldError> {
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
                .map_err(service::manifest_error(PathText(PACKAGE_FILE)))?;
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
    fn definitions(&self, loading: &mut Loading) -> Result<(), ScaffoldError> {
        while let Some(wanted) = next(loading) {
            let text = DefinitionText(
                self.packages
                    .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))?
                    .0,
            );
            let definition = self
                .manifest
                .definition(wanted.release.kind, &text)
                .map_err(service::manifest_error(PathText(&wanted.file.0)))?;
            definition_loaded(loading, &wanted, definition);
        }
        Ok(())
    }

    /// The workspace nearest `start`, read: its files, its packages and every definition its entries name.
    fn load(&self, start: &MachinePath) -> Result<Workspace, ScaffoldError> {
        let found = self.repository.find(start)?;
        let repository = &*found;
        let file = join(found.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let mut workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0))
            .map_err(service::manifest_error(PathText(&file.0)))?;
        let Role::Root(mut section) = workspace.role.clone() else {
            return Err(ScaffoldError::NoEntry {
                entry: file.0.to_string(),
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
        // Each entry is at what the lock pins, else at the latest its package offers.
        let pins = self.pins(found.as_ref())?;
        // A version the package cannot list is a failure, said once the pins are made.
        let failed = std::cell::RefCell::new(None::<PackagesError>);
        let version_of = |release: &Release| {
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
                            format!(
                                "{}/{}",
                                crate::domain::packages::service::kind_directory(release.kind),
                                release.name.0
                            )
                            .into(),
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
                    crate::domain::packages::service::latest(
                        &crate::domain::packages::types::VersionText(Box::default()),
                        offered.iter(),
                    )
                },
            )
        };
        crate::domain::entries::service::pin(&mut workspace.composition, &version_of);
        for each in &mut local_files {
            crate::domain::entries::service::pin(&mut each.composition, &version_of);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(error.into());
        }
        let LocalSplit {
            packages: local_packages,
            governed,
        } = local_split(
            files,
            package_paths(&packages, RepositoryText(&found.root().0)),
        );
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
        let WorkspaceLoad {
            workspace: loaded, ..
        } = finish(loading);
        Ok(Workspace { found, loaded })
    }

    /// Renders one template entry's file at `directory` with `values` and writes it when it does not exist.
    fn write(
        &self,
        workspace: &Workspace,
        placing: &Placing<'_>,
        entry: &TemplateEntry,
        _request: &ScaffoldRequest,
        report: &mut ScaffoldReport,
        staged: &mut Staging,
    ) -> Result<(), ScaffoldError> {
        let repository = Arc::clone(&workspace.found);
        let file = placed_name(&entry.file, placing.values);
        if let Some(capture) = unfilled(&file) {
            return Err(ScaffoldError::Capture {
                file: entry.file.0.to_string(),
                capture: capture.0,
            });
        }
        let path =
            join(placing.directory, PathText(&unescaped(Template(&file.0)).0)).unwrap_or_default();
        if repository.exists(&path) == Presence::Present {
            report.kept.push(path);
            return Ok(());
        }
        let Some(template) = workspace.loaded.templates.get(&template_release(entry)) else {
            return Err(ScaffoldError::NoEntry {
                entry: entry.template.0.to_string(),
            });
        };
        let draft = Draft {
            directory: placing.directory.clone(),
            values: placing.values.clone(),
            entry: entry.clone(),
        };
        if !staged.drafts.contains_key(&path) {
            staged.order.push(path.clone());
        }
        drop(staged.drafts.insert(path.clone(), draft));
        if matches!(entry.count, Count::When { .. }) {
            return Ok(());
        }
        let context = first_render(
            placing.values,
            &template.params,
            &entry.header.params,
            placing.directory,
            &*self.manifest,
        )
        .map_err(|MissingParam(param)| ScaffoldError::Param {
            file: path.0.to_string(),
            param,
        })?;
        let text = self
            .templates
            .render(&content(template), &context)
            .map_err(|source| ScaffoldError::Render {
                file: path.0.to_string(),
                source,
            })?;
        if text.0.trim().is_empty() {
            return Err(ScaffoldError::Unwritten {
                file: path.0.to_string(),
            });
        }
        drop(staged.texts.insert(path, text));
        Ok(())
    }

    /// Resolves conditions and render contexts against the complete prospective tree before publishing files.
    fn finish(
        &self,
        workspace: &Workspace,
        request: &ScaffoldRequest,
        report: &mut ScaffoldReport,
        staged: &mut Staging,
    ) -> Result<(), ScaffoldError> {
        let repository = Arc::clone(&workspace.found);
        let mut visited = BTreeSet::new();
        let limit = staged.drafts.len().saturating_mul(2).saturating_add(2);
        for _ in 0..limit {
            let state = staged
                .texts
                .iter()
                .map(|(path, text)| (path.0.clone(), text.0.clone()))
                .collect::<Vec<_>>();
            if !visited.insert(state) {
                return Err(ScaffoldError::Unsettled);
            }
            let mut loaded = workspace.loaded.clone();
            loaded.files.extend(staged.texts.keys().cloned());
            loaded.files.sort();
            loaded.files.dedup();
            let compile = self.compile();
            let placement = crate::domain::placement::service::place_at(
                &loaded,
                &compile,
                &staged.directories,
                &*self.manifest,
            )?;
            let holds = self.conditions(workspace, &loaded, &placement, staged)?;
            let next = self.render_drafts(&loaded, &placement, staged, &holds)?;
            if next == staged.texts {
                for path in &staged.order {
                    if let Some(text) = next.get(path) {
                        if request.writing == Writing::Write {
                            repository.write(path, text)?;
                        }
                        report.written.push(path.clone());
                    }
                }
                return Ok(());
            }
            staged.texts = next;
        }
        Err(ScaffoldError::Unsettled)
    }

    /// Evaluates the draft conditions against authored files and staged contents through the analysis port.
    fn conditions(
        &self,
        workspace: &Workspace,
        loaded: &LoadedWorkspace,
        placement: &Placement,
        staged: &Staging,
    ) -> Result<BTreeSet<ConditionAt>, ScaffoldError> {
        service::condition_parameters(loaded, placement, staged)?;
        let compile = self.compile();
        let sets =
            |globs: &[crate::domain::selectors::types::SetGlob]| self.matching.compile_set(globs);
        let plan = crate::domain::planning::service::plan(
            loaded,
            placement,
            RepoPath::default(),
            Compilers {
                compile: &compile,
                sets: &sets,
            },
        )?;
        let rules = plan
        .rules
        .iter()
        .filter(|rule| match plan.purposes.get(&rule.id) {
            Some(Purpose::Condition { scope, name, .. }) => staged.drafts.values().any(|draft| {
                placement
                    .scopes
                    .get(scope.0)
                    .is_some_and(|owner| draft.directory == owner.dir)
                    && matches!(&draft.entry.count, Count::When { condition, .. } if condition == name)
            }),
            _ => false,
        })
        .cloned()
        .collect::<Vec<_>>();
        let answer = if rules.is_empty() {
            crate::outbound::analysis::types::AnalysisReport::default()
        } else {
            self.analysis.lint(AnalysisRequest {
                root: workspace.found.root().clone(),
                rules,
                placed: plan.placed.clone(),
                corpus: Corpus::Listed(workspace.loaded.files.clone()),
                beside: staged
                    .texts
                    .iter()
                    .map(|(path, text)| GivenFile {
                        path: path.clone(),
                        text: crate::domain::contracts::types::RenderedContent(text.0.clone()),
                    })
                    .collect(),
                hosted: Hosted::Nothing,
            })?
        };
        if !answer.unknowns.is_empty() {
            return Err(ScaffoldError::Undecided {
                reason: answer
                    .unknowns
                    .iter()
                    .map(|unknown| unknown.message.0.as_ref())
                    .collect::<Vec<_>>()
                    .join(crate::domain::scaffolding::constants::CONDITION_SEPARATOR),
            });
        }
        Ok(answer
            .findings
            .iter()
            .filter_map(|finding| match plan.purposes.get(&finding.rule) {
                Some(Purpose::Condition { scope, name, .. }) => Some(ConditionAt {
                    scope: *scope,
                    condition: name.clone(),
                }),
                _ => None,
            })
            .collect::<BTreeSet<_>>())
    }

    /// Renders every required draft with the context of the prospective placement, without publishing it.
    fn render_drafts(
        &self,
        loaded: &LoadedWorkspace,
        placement: &Placement,
        staged: &Staging,
        holds: &BTreeSet<ConditionAt>,
    ) -> Result<BTreeMap<RepoPath, FileText>, ScaffoldError> {
        let mut next = BTreeMap::new();
        for (path, draft) in &staged.drafts {
            let template = loaded
                .templates
                .get(&template_release(&draft.entry))
                .ok_or_else(|| ScaffoldError::NoEntry {
                    entry: draft.entry.template.0.to_string(),
                })?;
            let owner = placement.scopes.iter().enumerate().find(|(_, scope)| {
                scope.dir == draft.directory
                    && composition_of(loaded, &scope.composition)
                        .templates
                        .iter()
                        .any(|entry| entry == &draft.entry)
            });
            if let Count::When {
                condition,
                then,
                otherwise,
            } = &draft.entry.count
            {
                let Some((index, _)) = owner else {
                    return Err(ScaffoldError::Undecided {
                        reason: format!("no placement for conditional file `{}`", path.0),
                    });
                };
                let count = if holds.contains(&ConditionAt {
                    scope: ScopeIndex(index),
                    condition: condition.clone(),
                }) {
                    then
                } else {
                    otherwise
                };
                if *count == Amount(0) {
                    continue;
                }
            }
            let mut context = first_render(
                &draft.values,
                &template.params,
                &draft.entry.header.params,
                &draft.directory,
                &*self.manifest,
            )
            .map_err(|MissingParam(param)| ScaffoldError::Param {
                file: path.0.to_string(),
                param,
            })?;
            if let Some((index, _)) = owner {
                let captures = placement
                    .placed
                    .get(path)
                    .map_or(&draft.values, |placed| &placed.captures);
                crate::domain::contexts::service::surroundings(
                    crate::domain::contexts::types::Surroundings {
                        workspace: loaded,
                        scopes: &placement.scopes,
                        matched: &placement.matched,
                        index: ScopeIndex(index),
                        captures,
                    },
                    &mut context,
                );
            }
            let text = self
                .templates
                .render(&content(template), &context)
                .map_err(|source| ScaffoldError::Render {
                    file: path.0.to_string(),
                    source,
                })?;
            if !text.0.trim().is_empty() {
                drop(next.insert(path.clone(), text));
            }
        }
        Ok(next)
    }

    /// Adds the names a template entry reads from what is set: those its patterns and params name, and those its
    /// template's content reads.
    fn template_entry_reads(
        &self,
        workspace: &Workspace,
        entry: &TemplateEntry,
        reads: &mut Reads,
    ) -> Result<(), ScaffoldError> {
        let template = workspace.loaded.templates.get(&template_release(entry));
        reads.0.extend(template_reads(entry, template).0);
        if let Some(template) = template {
            let read = self.templates.reads(&content(template)).map_err(|source| {
                ScaffoldError::Render {
                    file: entry.file.0.to_string(),
                    source,
                }
            })?;
            reads
                .0
                .extend(read.0.into_iter().map(|key| CaptureName(key.0)));
        }
        Ok(())
    }

    /// Adds the names the files of `composition` read from what is set, with those of the specs it nests, each spec once.
    fn composition_reads(
        &self,
        workspace: &Workspace,
        composition: &Composition,
        seen: &mut BTreeSet<Release>,
        reads: &mut Reads,
    ) -> Result<(), ScaffoldError> {
        reads.0.extend(condition_reads(composition).0);
        for entry in &composition.templates {
            self.template_entry_reads(workspace, entry, reads)?;
        }
        for entry in &composition.specs {
            let release = spec_release(entry);
            let Some(spec) = workspace.loaded.specs.get(&release) else {
                continue;
            };
            reads.0.extend(spec_reads(entry, spec).0);
            if seen.insert(release) {
                self.composition_reads(workspace, &spec.composition, seen, reads)?;
            }
        }
        Ok(())
    }

    /// Refuses the first name `--set` gives that nothing the template entry, or a condition beside it, reads.
    fn template_unread(
        &self,
        workspace: &Workspace,
        composition: &Composition,
        entry: &TemplateEntry,
        request: &ScaffoldRequest,
    ) -> Result<(), ScaffoldError> {
        let mut reads = condition_reads(composition);
        self.template_entry_reads(workspace, entry, &mut reads)?;
        service::unread(request, &reads)
    }

    /// Refuses the first name `--set` gives that nothing the spec entry's instance reads.
    fn spec_unread(
        &self,
        workspace: &Workspace,
        entry: &SpecEntry,
        spec: &crate::domain::definitions::types::SpecFile,
        request: &ScaffoldRequest,
    ) -> Result<(), ScaffoldError> {
        let mut reads = spec_reads(entry, spec);
        let mut seen = BTreeSet::from([spec_release(entry)]);
        self.composition_reads(workspace, &spec.composition, &mut seen, &mut reads)?;
        service::unread(request, &reads)
    }

    /// Writes the module file that declares the instance at `at`, beside its directory, when an entry of the workspace places it: it is
    /// written in the same call as the instance.
    fn module_file(
        &self,
        workspace: &Workspace,
        at: &RepoPath,
        captured: &Captures,
        request: &ScaffoldRequest,
        report: &mut ScaffoldReport,
        staged: &mut Staging,
    ) -> Result<(), ScaffoldError> {
        let compositions = std::iter::once((
            workspace.loaded.directory.clone(),
            &workspace.loaded.composition,
        ))
        .chain(
            workspace
                .loaded
                .locals
                .iter()
                .map(|local| (local.directory.clone(), &local.composition)),
        );
        for (base, outer) in compositions {
            for sibling in outer
                .templates
                .iter()
                .filter(|each| declaring(&base, each, captured, at) == Declaring::Declares)
            {
                let placing = Placing {
                    directory: &base,
                    values: captured,
                };
                self.write(workspace, &placing, sibling, request, report, staged)?;
            }
        }
        Ok(())
    }

    /// Scaffolds an entry of a spec's composition, below the workspace's own files, named as `explain` names it: a
    /// template entry of the instance at `at`, or the instance at `at` itself, which the spec entry of an enclosing
    /// instance places there because its `include` takes that directory. The placement decides both, through the same
    /// selectors as `check`, so a directory no entry takes is refused as for any name. None when no nested entry has the
    /// name.
    fn nested(
        &self,
        workspace: &Workspace,
        request: &ScaffoldRequest,
    ) -> Result<Option<ScaffoldReport>, ScaffoldError> {
        if std::path::Path::new(&*request.at.0).is_absolute() {
            return Err(ScaffoldError::InvalidPath {
                path: request.at.0.to_string(),
            });
        }
        let at = join(workspace.found.start(), PathText(&request.at.0)).ok_or_else(|| {
            ScaffoldError::InvalidPath {
                path: request.at.0.to_string(),
            }
        })?;
        let placement = crate::domain::placement::service::place_at(
            &workspace.loaded,
            &self.compile(),
            &BTreeSet::from([at.clone()]),
            &*self.manifest,
        )?;
        for scope in placement.scopes.iter().filter(|scope| scope.dir == at) {
            // What is set wins over what the instance's params and captures give.
            let mut values = given(&request.set);
            for (name, value) in scope.values.0.iter().chain(&scope.captures.0) {
                let _: &mut CaptureValue = values
                    .0
                    .entry(name.clone())
                    .or_insert_with(|| value.clone());
            }
            let placing = Placing {
                directory: &at,
                values: &values,
            };
            let mut report = ScaffoldReport {
                written: Vec::new(),
                kept: Vec::new(),
            };
            let mut staged = Staging::default();
            let composition = composition_of(&workspace.loaded, &scope.composition);
            if let Some(entry) = composition.templates.iter().find(|entry| {
                service::spellings(&entry.header, DefinitionKind::Template, &entry.template)
                    .iter()
                    .any(|spelled| *spelled.0 == *request.entry.0)
            }) {
                self.template_unread(workspace, composition, entry, request)?;
                self.write(
                    workspace,
                    &placing,
                    entry,
                    request,
                    &mut report,
                    &mut staged,
                )?;
                self.finish(workspace, request, &mut report, &mut staged)?;
                return Ok(Some(report));
            }
            let Origin::Instance { parent, entry } = scope.origin else {
                continue;
            };
            let Some(enclosing) = placement.scopes.get(parent.0) else {
                continue;
            };
            let outer = composition_of(&workspace.loaded, &enclosing.composition);
            let Some(placer) = outer.specs.get(entry.0) else {
                continue;
            };
            if !service::spellings(&placer.header, DefinitionKind::Spec, &placer.spec)
                .iter()
                .any(|spelled| *spelled.0 == *request.entry.0)
            {
                continue;
            }
            if let Some(spec) = workspace.loaded.specs.get(&spec_release(placer)) {
                self.spec_unread(workspace, placer, spec, request)?;
            }
            self.instance(
                workspace,
                &placing,
                composition,
                request,
                &mut report,
                &mut staged,
            )?;
            // The enclosing instance's file that declares this one, as `module_file` writes it for the workspace's own.
            for sibling in outer
                .templates
                .iter()
                .filter(|each| declaring(&enclosing.dir, each, &values, &at) == Declaring::Declares)
            {
                let beside = Placing {
                    directory: &enclosing.dir,
                    values: &values,
                };
                self.write(
                    workspace,
                    &beside,
                    sibling,
                    request,
                    &mut report,
                    &mut staged,
                )?;
            }
            self.finish(workspace, request, &mut report, &mut staged)?;
            return Ok(Some(report));
        }
        Ok(None)
    }

    /// Writes the files one instance of a spec places at `directory`, and those of the specs it places at literal paths.
    fn instance(
        &self,
        workspace: &Workspace,
        placing: &Placing<'_>,
        composition: &Composition,
        request: &ScaffoldRequest,
        report: &mut ScaffoldReport,
        staged: &mut Staging,
    ) -> Result<(), ScaffoldError> {
        let _: bool = staged.directories.insert(placing.directory.clone());
        // The captures the optional files of the spec introduce.
        let waits = waited(&composition.templates);
        for entry in &composition.templates {
            let optional = optionality(&entry.count);
            // An optional file is written only when its template says what a first writing starts it with.
            if optional == Optionality::Optional
                && !matches!(entry.count, Count::When { .. })
                && first_writing(workspace.loaded.templates.get(&template_release(entry)))
                    == FirstWriting::Unsaid
            {
                continue;
            }
            let placed = placed_name(&entry.file, placing.values);
            if naming(&placed) == Naming::Pattern {
                continue;
            }
            // A file whose path names a capture an optional file introduces waits for that file: a collection's items need the collection.
            if optional == Optionality::Required && waiting(&placed, &waits) == Waiting::Waits {
                continue;
            }
            // A spec's instance leaves out a file its template writes nothing into: its tool or its author writes it.
            match self.write(workspace, placing, entry, request, report, staged) {
                Ok(()) | Err(ScaffoldError::Unwritten { .. }) => {}
                Err(ScaffoldError::Capture { .. }) if optional == Optionality::Optional => {}
                Err(error) => return Err(error),
            }
        }
        for entry in &composition.specs {
            let Some(spec) = workspace.loaded.specs.get(&spec_release(entry)) else {
                continue;
            };
            for selector in &entry.header.include.0 {
                let crate::domain::selectors::types::Selector::Glob(pattern) = selector else {
                    continue;
                };
                let placed = placed_name(pattern, placing.values);
                if naming(&placed) == Naming::Pattern || fewest(&entry.count) == Amount(0) {
                    continue;
                }
                if let Some(capture) = unfilled(&placed) {
                    return Err(ScaffoldError::Capture {
                        file: pattern.0.to_string(),
                        capture: capture.0,
                    });
                }
                let inner = join(
                    placing.directory,
                    PathText(&unescaped(Template(&placed.0)).0),
                )
                .unwrap_or_default();
                let mut nested = placing.values.clone();
                nested.0.extend(
                    named_by_directory(&spec.params, &inner, &workspace.loaded.section.name).0,
                );
                nested
                    .0
                    .extend(literal_values(&entry.header.params, &nested, &*self.manifest).0);
                self.instance(
                    workspace,
                    &Placing {
                        directory: &inner,
                        values: &nested,
                    },
                    &spec.composition,
                    request,
                    report,
                    staged,
                )?;
            }
        }
        Ok(())
    }
}

impl ScaffoldUseCase for ScaffoldUseCaseImpl {
    fn scaffold(&self, request: ScaffoldRequest) -> Result<ScaffoldReport, ScaffoldError> {
        if let Some(capture) = service::capture_left(Template(&request.at.0)) {
            return Err(ScaffoldError::UnresolvedAt {
                at: request.at.0.to_string(),
                capture: capture.0.to_string(),
            });
        }
        let workspace = self.load(&request.start)?;
        let mut report = ScaffoldReport {
            written: Vec::new(),
            kept: Vec::new(),
        };
        let mut staged = Staging::default();
        let mut values = given(&request.set);
        let root = (
            workspace.loaded.directory.clone(),
            &workspace.loaded.composition,
        );
        let files = std::iter::once(root).chain(
            workspace
                .loaded
                .locals
                .iter()
                .map(|local| (local.directory.clone(), &local.composition)),
        );
        for (directory, composition) in files {
            if let Some(entry) = composition
                .templates
                .iter()
                .find(|entry| *reference(&entry.header, &entry.template) == request.entry)
            {
                self.template_unread(&workspace, composition, entry, &request)?;
                let placing = Placing {
                    directory: &directory,
                    values: &values,
                };
                self.write(
                    &workspace,
                    &placing,
                    entry,
                    &request,
                    &mut report,
                    &mut staged,
                )?;
                self.finish(&workspace, &request, &mut report, &mut staged)?;
                return Ok(report);
            }
            if let Some(entry) = composition
                .specs
                .iter()
                .find(|entry| *reference(&entry.header, &entry.spec) == request.entry)
            {
                let Some(spec) = workspace.loaded.specs.get(&spec_release(entry)) else {
                    continue;
                };
                let at = service::instance_at(&workspace, &request)?;
                // What is set wins over what the directory, the `include` and the entry's literals give.
                let named = named_by_directory(&spec.params, &at, &workspace.loaded.section.name);
                service::unset(&mut values, named);
                let captured = self.captures(entry, &directory, &at)?;
                self.spec_unread(&workspace, entry, spec, &request)?;
                service::unset(&mut values, captured.clone());
                let literals = literal_values(&entry.header.params, &values, &*self.manifest);
                service::unset(&mut values, literals);
                let placing = Placing {
                    directory: &at,
                    values: &values,
                };
                self.instance(
                    &workspace,
                    &placing,
                    &spec.composition,
                    &request,
                    &mut report,
                    &mut staged,
                )?;
                self.module_file(
                    &workspace,
                    &at,
                    &captured,
                    &request,
                    &mut report,
                    &mut staged,
                )?;
                self.finish(&workspace, &request, &mut report, &mut staged)?;
                return Ok(report);
            }
        }
        if let Some(nested) = self.nested(&workspace, &request)? {
            return Ok(nested);
        }
        Err(ScaffoldError::NoEntry {
            entry: request.entry.0.to_string(),
        })
    }
}
