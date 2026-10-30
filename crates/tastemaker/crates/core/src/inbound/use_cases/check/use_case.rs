// ignore: idiomatic.arch/file-length - check is one flow over eleven ports and services whose stages hand each other the one `Inputs` they read once, and what the steps decide (placement, planning, governance, linting, rendering, reporting) is already in the domain concepts it calls, so what is left is the order of the port calls and the state one call hands the next. The cuts that need fewer of the ports were measured: consulting validation's proofs (`consulted`, `stamp_of`, `content`: 140 lines over three ports) and digesting the files the kept answer rests on (`files`, `answered`: 175 lines) each fit an application service of their own, and with both out the use case still holds about 900 lines, so no one extraction reaches the limit; the rest splits only into a service for each stage of the flow, which is a redesign of this use case into stage services with a service test each, not a move of code.
//! The `check` use case: it reads the workspace's definitions once, places them, sends one lint call over its rules and,
//! when the plan holds counts or pairs, a second that judges them, and judges the answers. The answer is kept in the store with the digest of every file it rests on, so a run where none changed is a
//! lookup.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use rayon::prelude::*;

use super::errors::CheckError;
use super::interface::CheckUseCase;
use super::service::{checked_of, lint_answers, listing, manifest_error, package_of, reusable};
use super::types::{CheckReport, CheckRequest, Fixing, LintInputs, LockOutcome, PreparedPlan};
use crate::domain::applications::types::Plan;
use crate::domain::captures::types::Interpolations;
use crate::domain::contract_rendering::service::{rendered, requests};
use crate::domain::contract_rendering::types::{Rendered, Requested};
use crate::domain::definitions::constants::LOCK_FILE;
use crate::domain::definitions::service::{
    folded, governed, local, locals, package_paths, shared, variable_findings,
};
use crate::domain::definitions::types::{Inputs, LoadedWorkspace, LocalText, LockRead, Role};
use crate::domain::entries::service::pin;
use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::findings::types::{Finding, Report};
use crate::domain::governance::service::governance;
use crate::domain::linting::service::{
    lint_root, listed, made_lock, partitioned, refusals, runtime_refusal, within_paths,
};
use crate::domain::linting::types::{Partitioned, RefusalReason};
use crate::domain::loading::service::{begin, definition_read, finish, wanted};
use crate::domain::loading::types::Parsed;
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::service::{kind_directory, latest, lock, lock_status, locked_version};
use crate::domain::packages::types::{LockStatus, PackageRoot, Reach, ResolvedPackage, SourcePath};
use crate::domain::packages::types::{Ownership, PackageName, Release, VersionText};
use crate::domain::paths::service::{join, relative};
use crate::domain::paths::types::PathText;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::service::place;
use crate::domain::planning::service::plan;
use crate::domain::proofs::constants::UNPROVEN;
use crate::domain::reporting::types::{Reported, Reuse};
use crate::domain::scenario_proving::service::subject_of;
use crate::domain::selectors::service::{covers_any, first_match, resolve};
use crate::domain::selectors::types::{
    Coverage, PathTest, PatternFailure, SelectionContext, Spelling,
};
use crate::domain::variables::service::resolve as resolve_variables;
use crate::domain::writing::types::Writing;
use crate::outbound::analysis::errors::AnalysisError;
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{
    AnalysisReport, AnalysisRequest, Corpus, GivenFile, Hosted,
};
use crate::outbound::check_store::types::{Checked, CheckedFile, CheckedPath};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;

use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::{PackageFile, PackagedFile};
use crate::outbound::repository::interface::FoundRepository;
use crate::outbound::repository::interface::{Repository, RepositoryPort};
use crate::outbound::repository::types::Presence;

/// `check`, over the ports the edge injects.
pub struct CheckUseCaseImpl {
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Shapes and writes the report the invocation answers.
    reporting: Arc<crate::application::reporting::service::ReportingService>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Answers the analysis the definitions ask for.
    analysis: Arc<dyn AnalysisPort>,
    /// Answers the count and pair queries: an analysis that keeps nothing, so its call never rewrites what `analysis` kept
    /// for the next check.
    counting: Arc<dyn AnalysisPort>,
    /// The project's store: the check's answer.
    store: Arc<dyn crate::outbound::check_store::interface::CheckStorePort>,
    /// The project's store: the versions validation proved held, which `check` consults.
    proofs: Arc<dyn crate::outbound::validate_store::interface::ValidateStorePort>,
    /// Digests the files' contents.
    digest: Arc<dyn DigestPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
    /// Renders the templates' contents.
    templates: Arc<dyn crate::outbound::templates::interface::TemplatesPort>,
}

impl std::fmt::Debug for CheckUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The ports `check` reads and writes through.
pub struct CheckPorts {
    /// The repository filesystem.
    pub repository: Arc<dyn RepositoryPort>,
    /// Shared reporting operation assembled at the edge.
    pub reporting: Arc<crate::application::reporting::service::ReportingService>,
    /// The definitions' text.
    pub manifest: Arc<dyn ManifestPort>,
    /// The packages.
    pub packages: Arc<dyn PackagesPort>,
    /// ryo, keeping what the check's lint call leaves for the next.
    pub analysis: Arc<dyn AnalysisPort>,
    /// ryo, keeping nothing: it judges the counts and pairs the check's lint call left waiting.
    pub counting: Arc<dyn AnalysisPort>,
    /// The project's store: the check's answer.
    pub store: Arc<dyn crate::outbound::check_store::interface::CheckStorePort>,
    /// The project's store: the versions validation proved held.
    pub proofs: Arc<dyn crate::outbound::validate_store::interface::ValidateStorePort>,
    /// The digests.
    pub digest: Arc<dyn DigestPort>,
    /// The pattern compiler.
    pub matching: Arc<dyn MatchingPort>,
    /// The template engine, rendering the contents the placed files are compared with.
    pub templates: Arc<dyn crate::outbound::templates::interface::TemplatesPort>,
}

impl std::fmt::Debug for CheckPorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CheckUseCaseImpl {
    /// The lock `read`, parsed; none when there is no lock. A lock that does not read is an error to say, never a lock to
    /// treat as missing and rewrite.
    fn parsed_lock(
        &self,
        read: &LockRead,
    ) -> Result<Option<crate::domain::packages::types::Lock>, CheckError> {
        match read {
            LockRead::Read(text) => self
                .manifest
                .lock(&DefinitionText(text.0.clone()))
                .map(Some)
                .map_err(|source| CheckError::Manifest {
                    file: LOCK_FILE.to_owned(),
                    source,
                }),
            LockRead::Absent => Ok(None),
        }
    }

    /// The role of the `tastemaker.toml` at `path`, root or local, or why it does not read. Only a root file below the
    /// workspace makes a nested workspace whose tree it does not govern.
    fn role_of(&self, repository: &dyn Repository, path: &RepoPath) -> Result<Role, CheckError> {
        let text = repository.read(path)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(|source| CheckError::Manifest {
                file: path.0.to_string(),
                source,
            })
    }

    /// `check` over the ports the edge injects.
    #[must_use]
    pub fn new(ports: CheckPorts) -> Self {
        Self {
            repository: ports.repository,
            reporting: ports.reporting,
            manifest: ports.manifest,
            packages: ports.packages,
            analysis: ports.analysis,
            counting: ports.counting,
            store: ports.store,
            proofs: ports.proofs,
            digest: ports.digest,
            matching: ports.matching,
            templates: ports.templates,
        }
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

    /// The lock as the workspace holds it: read, or absent when there is none or `--fix` is to write it again.
    fn lock_read(
        &self,
        request: &CheckRequest,
        found: &dyn FoundRepository,
        lock_file: &RepoPath,
    ) -> Result<LockRead, CheckError> {
        let repository = found;
        Ok(match repository.exists(lock_file) {
            Presence::Present => {
                let text = repository.read(lock_file)?;
                // A lock that does not begin with the version this build writes is the one `--fix` writes again, as its
                // error says: it is read as no lock, and rewritten. Without `--fix` it is the error.
                let mended = request.fixing == Fixing::Fix
                    && matches!(
                        self.manifest.lock(&DefinitionText(text.0.clone())),
                        Err(crate::outbound::manifest::errors::ManifestError::LockVersion { .. })
                    );
                if mended {
                    LockRead::Absent
                } else {
                    LockRead::Read(text)
                }
            }
            Presence::Absent => LockRead::Absent,
        })
    }

    /// Reads the root `tastemaker.toml`, the lock, the files the workspace governs and its local files.
    fn inputs(
        &self,
        request: &CheckRequest,
        found: &dyn FoundRepository,
    ) -> Result<Inputs, CheckError> {
        let repository = found;
        let file = join(
            found.workspace(),
            PathText(crate::domain::definitions::constants::WORKSPACE_FILE),
        )
        .unwrap_or_default();
        let root_text = repository.read(&file)?;
        let mut workspace = self
            .manifest
            .workspace_file(&DefinitionText(root_text.0.clone()))
            .map_err(manifest_error(&file))?;
        let Role::Root(mut section) = workspace.role.clone() else {
            return Err(CheckError::NotARoot {
                file: file.0.to_string(),
            });
        };
        let mut problems = resolve_variables(
            &mut workspace.composition,
            &mut [&mut section.rules_exclude, &mut section.members],
        );
        problems.sort();
        workspace.role = Role::Root(section.clone());
        let lock_file = join(found.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
        let lock_text = self.lock_read(request, found, &lock_file)?;
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
            CheckError::Placement(crate::domain::placement::errors::PlacementError::Selector {
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
        let super::types::Listing {
            files: listed,
            links,
        } = listing(repository, std::slice::from_ref(found.workspace()))?;
        let files = governed(listed, &file, member, |path: &RepoPath| {
            self.role_of(repository, path)
        })?;
        let mut read_locals = Vec::new();
        for path in locals(&files, &file, member, |path| covers_any(&excluded, path)) {
            read_locals.push(LocalText {
                path: path.clone(),
                text: repository.read(path)?,
            });
        }
        let within = within_paths(found.start(), &request.within);
        // A path that holds no file of the workspace is a mistyped or missing path, said rather than checked as nothing.
        if let Some(path) = within.iter().find(|path| {
            !path.0.is_empty()
                && !files.iter().any(|file_path| {
                    file_path.0 == path.0
                        || file_path
                            .0
                            .strip_prefix(&*path.0)
                            .is_some_and(|rest| rest.starts_with('/'))
                })
        }) {
            return Err(CheckError::Within {
                path: path.0.to_string(),
            });
        }
        Ok(Inputs {
            within,
            repository: found.root().clone(),
            directory: found.workspace().clone(),
            file,
            root_text,
            lock_text,
            files,
            links,
            locals: read_locals,
            section,
            workspace,
            variables: problems,
        })
    }

    /// Resolves the workspace's packages, direct and transitive, breadth first.
    fn packages(
        &self,
        inputs: &Inputs,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, CheckError> {
        let workspace_directory = PackageRoot(
            std::path::Path::new(&*inputs.repository.0)
                .join(&*inputs.directory.0)
                .display()
                .to_string()
                .into(),
        );
        self.resolved(&inputs.section, &workspace_directory)
    }

    /// The packages `section` depends on, direct and transitive, breadth first, its relative paths taken from `directory`.
    fn resolved(
        &self,
        section: &crate::domain::definitions::types::WorkspaceSection,
        workspace_directory: &PackageRoot,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, CheckError> {
        let mut queue = section
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.clone(),
                    workspace_directory.clone(),
                    SourcePath(Box::default()),
                    Reach::Direct,
                )
            })
            .collect::<VecDeque<_>>();
        let mut packages = BTreeMap::new();
        while let Some((dependency, from, base, reach)) = queue.pop_front() {
            if packages.contains_key(&dependency.package) {
                continue;
            }
            let located = self.packages.locate(&dependency, &from, &base)?;
            let text = self
                .packages
                .read(&located.directory, &PackageFile(PACKAGE_FILE.into()))?;
            let manifest =
                self.manifest
                    .package(&DefinitionText(text.0))
                    .map_err(manifest_error(&RepoPath(
                        format!("{}/{PACKAGE_FILE}", located.directory.0).into(),
                    )))?;
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

    /// Reads each local `tastemaker.toml`: its variables resolved, its folded entries and variable problems found.
    fn local_files(
        &self,
        inputs: &Inputs,
        findings: &mut Vec<Finding>,
    ) -> Result<Vec<crate::domain::definitions::types::LocalFile>, CheckError> {
        let mut local_files = Vec::new();
        for LocalText { path, text } in &inputs.locals {
            let mut file = self
                .manifest
                .workspace_file(&DefinitionText(text.0.clone()))
                .map_err(manifest_error(path))?;
            if let Role::Root(_) = file.role {
                return Err(CheckError::Resolution {
                    message: format!(
                        "`{}`: a local tastemaker.toml never has a `workspace`: the root file declares it",
                        path.0
                    ),
                });
            }
            findings.extend(variable_findings(
                &resolve_variables(&mut file.composition, &mut []),
                path,
            ));
            findings.extend(folded(&file.composition, path));
            local_files.push(local(path, file.composition));
        }
        Ok(local_files)
    }

    /// Reads every definition the load asks for, until it asks for none; a spec's variable problems are findings. Each
    /// wave of definitions is read and parsed in parallel, then recorded in the order the load asked for them.
    fn definitions(
        &self,
        loading: &mut crate::domain::loading::types::Loading,
        findings: &mut Vec<Finding>,
    ) -> Result<(), CheckError> {
        loop {
            let wave = wanted(loading);
            if wave.is_empty() {
                return Ok(());
            }
            let outcomes = wave
                .par_iter()
                .map(|wanted| self.definition(wanted))
                .collect::<Vec<_>>();
            for (wanted, outcome) in wave.iter().zip(outcomes) {
                let Parsed { definition, place } = outcome?;
                let problems = definition_read(loading, wanted, definition);
                findings.extend(variable_findings(&problems, &place));
            }
        }
    }

    /// One definition read and parsed, and the content file a template names.
    fn definition(
        &self,
        wanted: &crate::domain::loading::types::Wanted,
    ) -> Result<Parsed, CheckError> {
        let text = DefinitionText(
            self.packages
                .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))?
                .0,
        );
        let place = RepoPath(format!("{}/{}", wanted.directory.0, wanted.file.0).into());
        let definition = self
            .manifest
            .definition(wanted.release.kind, &text)
            .map_err(manifest_error(&place))?;
        Ok(Parsed { definition, place })
    }

    /// The version an entry naming `release` uses: what `lock` pins for its definition, else the latest its package
    /// offers.
    fn version_of(
        &self,
        lock: Option<&crate::domain::packages::types::Lock>,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        release: &Release,
    ) -> Result<VersionText, PackagesError> {
        if let Some(version) = lock.and_then(|lock| locked_version(lock, release)) {
            return Ok(version);
        }
        let Some(package) = packages.get(&release.package) else {
            return Ok(VersionText(Box::default()));
        };
        let offered = self.packages.versions(
            &package.directory,
            &PackageFile(format!("{}/{}", kind_directory(release.kind), release.name.0).into()),
        )?;
        Ok(latest(&VersionText(Box::default()), offered.iter()))
    }

    /// The latest version each package offers of every definition the workspace reads, for `#outdated`.
    #[expect(
        clippy::indexing_slicing,
        reason = "a shared release is loaded from its package, so the package is among the workspace's: a missing one is a bug that must crash, never an unjudged #outdated"
    )]
    fn latest_versions(&self, workspace: &mut LoadedWorkspace) -> Result<(), PackagesError> {
        for release in shared(workspace) {
            // A shared release is loaded from its package, so the package is among the workspace's: indexing crashes on a
            // missing one rather than leave the release without a latest version and its `#outdated` unjudged.
            let package = &workspace.packages[&release.package];
            let offered = self.packages.versions(
                &package.directory,
                &PackageFile(format!("{}/{}", kind_directory(release.kind), release.name.0).into()),
            )?;
            let best = latest(&release.version, offered.iter());
            drop(workspace.latest.insert(release, best));
        }
        Ok(())
    }

    /// The placed files compared with their templates' contents, each content rendered with the context its placement
    /// gives it, in parallel: the contracts, and the `#template` errors of the contents that could not be rendered.
    fn contracts(
        &self,
        workspace: &LoadedWorkspace,
        placement: &crate::domain::placement::types::Placement,
        failures: &mut Vec<Finding>,
    ) -> Vec<crate::domain::contracts::types::Contract> {
        let made = requests(workspace, placement, &*self.manifest)
            .into_par_iter()
            .map(|requested| match requested {
                Requested::Render(request) => {
                    match self.templates.render(&request.content, &request.context) {
                        Ok(text) => rendered(*request, Ok(text)),
                        Err(reason) => rendered(*request, Err(&reason)),
                    }
                }
                Requested::Failed(finding) => Rendered::Failed(finding),
            })
            .collect::<Vec<_>>();
        let mut contracts = Vec::new();
        for each in made {
            match each {
                Rendered::Contract(contract) => contracts.push(contract),
                Rendered::Failed(finding) => failures.push(finding),
                Rendered::Nothing => {}
            }
        }
        contracts
    }

    /// The lint call over `plan`'s rules, but those `refused`: a rule this ryo cannot run is refused with a `#query` error in
    /// `findings`, unless it governs none of the files read, and the call is made again without it.
    fn lint(
        &self,
        plan: &Plan,
        LintInputs {
            root,
            files: corpus,
        }: LintInputs<'_>,
        findings: &mut Vec<Finding>,
        refused: &mut BTreeSet<crate::domain::applications::types::LintRuleId>,
    ) -> Result<AnalysisReport, CheckError> {
        loop {
            let rules = plan
                .rules
                .iter()
                .filter(|rule| !refused.contains(&rule.id))
                .cloned()
                .collect::<Vec<_>>();
            match self.analysis.lint(AnalysisRequest {
                root: root.clone(),
                rules,
                placed: plan.placed.clone(),
                corpus: Corpus::Listed(corpus.to_vec()),
                // The rendered contents lie beneath the lint call's root, and ryo reads every path relative to it.
                beside: plan
                    .contracts
                    .iter()
                    .filter_map(|contract| {
                        relative(&contract.rendered, &plan.base).map(|rest| GivenFile {
                            path: RepoPath(rest.0.into()),
                            text: contract.text.clone(),
                        })
                    })
                    .collect(),
                hosted: Hosted::Nothing,
            }) {
                Ok(report) => return Ok(report),
                Err(AnalysisError::Rule { rule, reason })
                    if !refused.contains(&rule) && plan.purposes.contains_key(&rule) =>
                {
                    // A rule that governs none of the files read has nothing to find: it is left out, and said so at
                    // `info`, not refused.
                    let sets = self.sets();
                    findings.extend(runtime_refusal(
                        plan,
                        &rule,
                        &RefusalReason(reason.into()),
                        governance(plan, &rule, corpus, &sets),
                    ));
                    let _: bool = refused.insert(rule);
                }
                Err(other) => return Err(other.into()),
            }
        }
    }

    /// The findings of `plan`'s counts and pairs: a second lint call, over the rows they host once the conditions they
    /// read have answered in `report`, its corpus empty; none when the plan holds neither.
    fn counted(
        &self,
        plan: &Plan,
        root: &MachinePath,
        report: &AnalysisReport,
        refused: &BTreeSet<crate::domain::applications::types::LintRuleId>,
    ) -> Result<Vec<Finding>, CheckError> {
        let holds = crate::domain::judging::service::holds(plan, refused, &report.findings);
        let rows = crate::domain::judging::service::host(
            &plan.counts,
            &plan.pairs,
            &holds,
            &plan.closings,
        );
        if rows.entries.is_empty() && rows.closings.candidates.is_empty() {
            return Ok(Vec::new());
        }
        let counted = self.counting.lint(AnalysisRequest {
            root: root.clone(),
            rules: crate::domain::judging::service::rules(),
            placed: Vec::new(),
            corpus: Corpus::Given(Vec::new()),
            beside: Vec::new(),
            hosted: Hosted::Counts(rows),
        })?;
        let judgments = counted
            .findings
            .iter()
            .filter_map(crate::domain::judging::service::judgment_of)
            .collect::<Vec<_>>();
        Ok(crate::domain::judging::service::findings(
            &plan.counts,
            &plan.pairs,
            &plan.closings,
            &plan.excluded,
            &judgments,
        ))
    }

    /// Reads every definition the workspace names, and prepares the plan.
    fn prepare(&self, inputs: &Inputs) -> Result<PreparedPlan, CheckError> {
        let loading_span = tracing::info_span!("load").entered();
        let packages = self.packages(inputs)?;
        let mut findings = variable_findings(&inputs.variables, &inputs.file);
        findings.extend(folded(&inputs.workspace.composition, &inputs.file));
        let mut local_files = self.local_files(inputs, &mut findings)?;
        let pins = self.parsed_lock(&inputs.lock_text)?;
        // A version the package cannot list is a failure, said once the pins are made.
        let failed = std::cell::RefCell::new(None::<PackagesError>);
        let version_of =
            |release: &Release| match self.version_of(pins.as_ref(), &packages, release) {
                Ok(version) => version,
                Err(error) => {
                    if failed.borrow().is_none() {
                        *failed.borrow_mut() = Some(error);
                    }
                    VersionText(Box::default())
                }
            };
        let mut composition = inputs.workspace.composition.clone();
        pin(&mut composition, &version_of);
        for local in &mut local_files {
            pin(&mut local.composition, &version_of);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(error.into());
        }
        let package_dirs = package_paths(
            &packages,
            crate::domain::definitions::types::RepositoryText(&inputs.repository.0),
        );
        let Partitioned {
            local_packages,
            governed,
        } = partitioned(&inputs.files, &package_dirs);
        let mut loading = begin(LoadedWorkspace {
            file: inputs.file.clone(),
            directory: inputs.directory.clone(),
            section: inputs.section.clone(),
            composition,
            locals: local_files,
            packages,
            specs: BTreeMap::new(),
            rules: BTreeMap::new(),
            templates: BTreeMap::new(),
            latest: BTreeMap::new(),
            files: governed,
            local_packages,
        });
        self.definitions(&mut loading, &mut findings)?;
        let crate::domain::loading::types::WorkspaceLoad {
            mut workspace,
            mut read,
            problems,
        } = finish(loading);
        if let Some(problem) = problems.first() {
            return Err(CheckError::Resolution {
                message: problem.0.to_string(),
            });
        }
        self.latest_versions(&mut workspace)?;
        let direct = inputs
            .section
            .dependencies
            .iter()
            .map(|dependency| dependency.package.clone())
            .collect::<Vec<_>>();
        let made = lock(&direct, &workspace.packages, shared(&workspace).iter());
        drop(loading_span);
        let compile = self.compile();
        let placing = tracing::info_span!("place").entered();
        let placement = place(&workspace, &compile, &*self.manifest)?;
        drop(placing);
        let planning = tracing::info_span!("plan").entered();
        // The lint call is rooted at the workspace's directory, the one base every path of its files has: a rule reads the
        // paths of the files it is handed relative to it, the same in a repository of its own or nested in another.
        let base = workspace.directory.clone();
        let mut plan = plan(
            &workspace,
            &placement,
            base,
            crate::domain::applications::types::Compilers {
                compile: &compile,
                sets: &self.sets(),
            },
        )?;
        let contracts = self.contracts(&workspace, &placement, &mut findings);
        crate::domain::built_ins::service::compared(&mut plan, contracts);
        drop(planning);
        // Dependency manifests choose the versions and package roots used by the plan.
        read.extend(workspace.packages.values().map(|package| {
            crate::domain::loading::types::ReadFile {
                directory: package.directory.clone(),
                file: crate::domain::loading::types::DefinitionFile(PACKAGE_FILE.into()),
            }
        }));
        Ok(PreparedPlan {
            plan,
            lock: made.packages,
            findings,
            read,
        })
    }

    /// The lock this load makes, against the one on disk: current, rewritten under `--fix`, else an error.
    fn lock_outcome(
        &self,
        repository: &dyn Repository,
        inputs: &Inputs,
        kept: &PreparedPlan,
        fixing: Fixing,
        writing: Writing,
    ) -> Result<LockOutcome, CheckError> {
        // The lock this load makes, against the one on disk.
        let made = made_lock(&inputs.section, kept.lock.clone());
        let prior = self.parsed_lock(&inputs.lock_text)?;
        let lock_file = join(&inputs.directory, PathText(LOCK_FILE)).unwrap_or_default();
        Ok(match lock_status(prior.as_ref(), &made) {
            LockStatus::Current => LockOutcome::Current,
            status @ (LockStatus::Stale | LockStatus::Missing) => match fixing {
                Fixing::Fix if writing == Writing::DryRun => LockOutcome::Planned,
                Fixing::Fix => {
                    let text = self.manifest.lock_text(&made);
                    repository.write(&lock_file, &FileText(text.0))?;
                    LockOutcome::Rewritten
                }
                Fixing::Report => {
                    return Err(CheckError::Lock {
                        file: lock_file.0.to_string(),
                        state: if status == LockStatus::Missing {
                            crate::domain::packages::constants::LOCK_MISSING
                        } else {
                            crate::domain::packages::constants::LOCK_STALE
                        }
                        .to_owned(),
                    });
                }
            },
        })
    }

    /// Consults validation's proofs: every version the lock pins must have its row in validation's `definitions`
    /// table, under the digest of its files and its uses' files as they are now.
    fn consulted(
        &self,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        lock: &[crate::domain::packages::types::LockedPackage],
    ) -> Result<(), CheckError> {
        use crate::domain::proofs::service::{node_parts, use_text};
        use crate::domain::proofs::types::{NodeName, NodePart, UseText};
        let rows = self
            .proofs
            .definitions()
            .0
            .into_iter()
            .map(|row| (row.release.clone(), row))
            .collect::<BTreeMap<_, _>>();
        let pinned = lock
            .iter()
            .flat_map(|locked| {
                locked.releases.iter().map(|used| Release {
                    package: locked.name.clone(),
                    kind: used.kind,
                    name: used.name.clone(),
                    version: used.version.clone(),
                    ownership: Ownership::Shared,
                })
            })
            .collect::<Vec<_>>();
        // Every version read, its uses among them, digested once and at once.
        let read = pinned
            .iter()
            .filter_map(|release| rows.get(release))
            .flat_map(|row| std::iter::once(&row.release).chain(&row.uses))
            .collect::<BTreeSet<_>>();
        let mut listings = BTreeMap::new();
        for release in &read {
            if let Some(directory) = package_of(release, packages)
                && !listings.contains_key(&directory)
                && let Ok(listed) = self
                    .packages
                    .files(&directory, &PackageFile(Box::default()))
            {
                drop(listings.insert(directory, listed.0));
            }
        }
        // A version whose stamp, its uses' with it, is the one its row was kept with holds unread.
        let stamps = read
            .par_iter()
            .filter_map(|release| Some((*release, self.stamp_of(release, packages, &listings)?)))
            .collect::<BTreeMap<_, _>>();
        let fresh = pinned
            .iter()
            .filter_map(|release| rows.get(release))
            .filter(|row| {
                std::iter::once(&row.release)
                    .chain(&row.uses)
                    .map(|each| stamps.get(each).copied())
                    .collect::<Option<Vec<_>>>()
                    .is_some_and(|each| {
                        crate::domain::proofs::service::summary(&each, &[]) == row.stamp
                    })
            })
            .map(|row| row.release.clone())
            .collect::<BTreeSet<_>>();
        let unread = pinned
            .iter()
            .filter_map(|release| rows.get(release))
            .filter(|row| !fresh.contains(&row.release))
            .flat_map(|row| std::iter::once(&row.release).chain(&row.uses))
            .collect::<BTreeSet<_>>();
        let contents = unread
            .into_par_iter()
            .filter_map(|release| Some((release, self.content(release, packages, &listings)?)))
            .collect::<BTreeMap<_, _>>();
        for release in pinned {
            let stamped = fresh.contains(&release);
            let held = stamped
                || rows.get(&release).is_some_and(|row| {
                    let mut parts = Vec::with_capacity(row.uses.len() + 1);
                    for each in std::iter::once(&release).chain(&row.uses) {
                        let Some(content) = contents.get(each) else {
                            return false;
                        };
                        let name = if *each == release {
                            UseText::default()
                        } else {
                            use_text(each)
                        };
                        parts.push((name, content));
                    }
                    self.digest.digest(DigestParts(
                        &node_parts(parts.iter().map(|(name, digest)| NodePart {
                            name: NodeName(&name.0),
                            digest,
                        }))
                        .0,
                    )) == row.digest
                });
            if !held {
                return Err(CheckError::Proof {
                    subject: subject_of(&release).id.0.to_string(),
                    reason: UNPROVEN.to_owned(),
                });
            }
        }
        Ok(())
    }

    /// The stamp of the version `release` as its files are now, as validation takes it, its package's files as `listings`
    /// lists them; none when its package is not listed.
    fn stamp_of(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listings: &BTreeMap<PackageRoot, Vec<PackageFile>>,
    ) -> Option<crate::domain::files::types::Stamp> {
        use crate::domain::proofs::service::{summary, version_paths};
        let directory = package_of(release, packages)?;
        let scope = crate::domain::scenario_proving::service::release_directory(release).0;
        let paths = version_paths(
            listings
                .get(&directory)?
                .iter()
                .map(|file| PathText(&file.0)),
            PathText(&scope),
        );
        let asked = paths
            .files
            .iter()
            .chain(&paths.directories)
            .map(|path| PackagedFile {
                directory: directory.clone(),
                file: PackageFile(path.0.into()),
            })
            .collect::<Vec<_>>();
        let stamps = self.packages.stamps(&asked);
        let (of_files, of_directories) = stamps.0.split_at(paths.files.len().min(stamps.0.len()));
        Some(summary(of_files, of_directories))
    }

    /// The digest of the files of the version `release` as they are now, each by its path, as validation digests them,
    /// its package's files as `listings` lists them; none when its package is not listed or a file does not read.
    fn content(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listings: &BTreeMap<PackageRoot, Vec<PackageFile>>,
    ) -> Option<crate::domain::digests::types::Digest> {
        use crate::domain::proofs::service::node_parts;
        use crate::domain::proofs::types::{NodeName, NodePart};
        let directory = package_of(release, packages)?;
        let prefix = format!(
            "{}/",
            crate::domain::scenario_proving::service::release_directory(release).0
        );
        let mut read = listings
            .get(&directory)?
            .iter()
            .filter(|file| file.0.starts_with(&prefix))
            .map(|file| {
                let text = self.packages.read(&directory, file).ok()?; // ignore: idiomatic.arch/no-ok-on-result - a version whose file does not read has no digest (the doc above), so its proof is not reused and validation reads it again
                Some((
                    &*file.0,
                    self.digest.digest(DigestParts(&[text.0.as_bytes()])),
                ))
            })
            .collect::<Option<Vec<_>>>()?;
        read.sort_by(|left, right| left.0.cmp(right.0));
        Some(
            self.digest.digest(DigestParts(
                &node_parts(read.iter().map(|(path, digest)| NodePart {
                    name: NodeName(path),
                    digest,
                }))
                .0,
            )),
        )
    }

    /// Every file the answer rests on, with the digest of its text now: the files the workspace governs, its root file
    /// and its lock, by their paths in the repository, and the definition files of its packages `read` names, by their
    /// paths on the machine. A file whose stamp is as `kept` holds it keeps its digest unread; a symbolic link is never
    /// read; a file that does not read as text has an empty digest.
    fn files(
        &self,
        repository: &dyn Repository,
        inputs: &Inputs,
        read: &[PackagedFile],
        kept: &[CheckedFile],
    ) -> Vec<CheckedFile> {
        use crate::domain::files::types::Stamp;
        let lock = join(&inputs.directory, PathText(LOCK_FILE)).unwrap_or_default();
        let mut paths = inputs
            .files
            .iter()
            .chain([&inputs.file, &lock])
            .cloned()
            .collect::<Vec<_>>();
        paths.sort();
        paths.dedup();
        let known = kept
            .iter()
            .map(|file| (&*file.path.0, file))
            .collect::<BTreeMap<_, _>>();
        // A file whose stamp did not move keeps its digest; any other is read and digested again.
        let digested = |path: CheckedPath, stamp: Stamp, text: &dyn Fn() -> Option<FileText>| {
            let digest = match known.get(&*path.0) {
                Some(file) if file.stamp == stamp && stamp != Stamp::default() => {
                    file.digest.clone()
                }
                _ => text().map_or_else(crate::domain::digests::types::Digest::default, |text| {
                    self.digest.digest(DigestParts(&[text.0.as_bytes()]))
                }),
            };
            CheckedFile {
                path,
                digest,
                stamp,
            }
        };
        let stamps = repository.stamps(&paths);
        let mut files = paths
            .par_iter()
            .zip(stamps.0.par_iter())
            .map(|(path, stamp)| {
                if inputs.links.contains(path) {
                    CheckedFile {
                        path: CheckedPath(path.0.clone()),
                        digest: crate::domain::digests::types::Digest::default(),
                        stamp: Stamp::default(),
                    }
                } else {
                    digested(CheckedPath(path.0.clone()), *stamp, &|| {
                        repository.read(path).ok() // ignore: idiomatic.arch/no-ok-on-result - a file that does not read digests as no file, so the kept answer is not reused and the check reads it again
                    })
                }
            })
            .collect::<Vec<_>>();
        let package_stamps = self.packages.stamps(read);
        files.par_extend(read.par_iter().zip(package_stamps.0.par_iter()).map(
            |(packaged, stamp)| {
                digested(
                    CheckedPath(
                        std::path::Path::new(&*packaged.directory.0)
                            .join(&*packaged.file.0)
                            .display()
                            .to_string()
                            .into(),
                    ),
                    *stamp,
                    &|| self.packages.read(&packaged.directory, &packaged.file).ok(), // ignore: idiomatic.arch/no-ok-on-result - a package file that does not read digests as no file, so the kept answer is not reused and the check reads it again
                )
            },
        ));
        files.sort();
        files.dedup();
        files
    }

    /// The answer the last check kept, when the check is unscoped and every file it rests on is as it was, and the
    /// versions the lock pins are proven; none when it must check again.
    fn answered(
        &self,
        repository: &dyn Repository,
        inputs: &Inputs,
        kept: &Checked,
    ) -> Result<Option<Report>, CheckError> {
        if !inputs.within.is_empty() {
            return Ok(None);
        }
        // A package's file is kept by its path on the machine: the package it is in is the one whose directory is
        // the longest that holds it.
        let packages = if kept.files.is_empty() {
            None
        } else {
            self.packages(inputs).ok() // ignore: idiomatic.arch/no-ok-on-result - packages that do not resolve leave no kept answer to reuse: the check runs, and resolving them there reports why
        };
        let read = packages.as_ref().map(|packages| {
            kept.files
                .iter()
                .filter_map(|file| {
                    packages
                        .values()
                        .filter_map(|package| {
                            let rest = file
                                .path
                                .0
                                .strip_prefix(&*package.directory.0)?
                                .strip_prefix('/')?;
                            Some(PackagedFile {
                                directory: package.directory.clone(),
                                file: PackageFile(rest.into()),
                            })
                        })
                        .max_by_key(|packaged| packaged.directory.0.len())
                })
                .collect::<Vec<_>>()
        });
        if let (Some(packages), Some(read)) = (&packages, &read)
            && self.files(repository, inputs, read, &kept.files) == kept.files
        {
            let lock = self
                .parsed_lock(&inputs.lock_text)?
                .map(|read_lock| read_lock.packages)
                .unwrap_or_default();
            self.consulted(packages, &lock)?;
            return Ok(Some(Report {
                workspace: inputs.section.name.clone(),
                findings: kept.findings.clone(),
                suppressed: kept.suppressed.clone(),
                evaluated: kept.evaluated.clone(),
                sourced: kept
                    .sourced
                    .iter()
                    .map(|each| (RepoPath(each.path.0.clone()), each.incoming))
                    .collect(),
            }));
        }
        Ok(None)
    }

    /// Turns the lint call's answer into the report, the files a finding is placed in read from the repository.
    fn judge(
        &self,
        repository: &dyn Repository,
        inputs: &Inputs,
        plan: &Plan,
        report: AnalysisReport,
        findings: Vec<Finding>,
        reuse: &Reuse,
    ) -> Result<Report, CheckError> {
        let report = self.reporting.prepare(
            repository,
            Reported {
                plan,
                answers: lint_answers(plan, report),
                findings,
                within: &inputs.within,
                reuse,
            },
        )?;
        Ok(report)
    }

    /// The rules the ignores of a file are read against, as one digest: an ignore that names a rule no longer there is at fault.
    fn known_digest(&self, plan: &Plan) -> crate::domain::digests::types::Digest {
        let parts = plan
            .known
            .iter()
            .map(|rule| rule.0.as_bytes())
            .collect::<Vec<_>>();
        self.digest.digest(DigestParts(&parts))
    }
}

impl CheckUseCase for CheckUseCaseImpl {
    fn check(&self, request: CheckRequest) -> Result<CheckReport, CheckError> {
        let preparing = tracing::info_span!("prepare").entered();
        let reading = tracing::info_span!("read_inputs").entered();
        let found = self.repository.find(&request.start)?;
        let repository = &*found;
        let mut inputs = self.inputs(&request, &*found)?;
        drop(reading);
        // While no file the last answer rests on changed, the answer is the last.
        // A scoped check neither reuses nor keeps an answer, so it reads none.
        let whole = inputs.within.is_empty();
        let kept = if whole {
            self.store.checked()
        } else {
            Checked::default()
        };
        if let Some(report) = self.answered(repository, &inputs, &kept)? {
            return Ok(CheckReport {
                report,
                repository: inputs.repository,
                lock: LockOutcome::Current,
            });
        }
        let mut prepared = self.prepare(&inputs)?;
        let lock_outcome = self.lock_outcome(
            repository,
            &inputs,
            &prepared,
            request.fixing,
            request.writing,
        )?;
        if lock_outcome == LockOutcome::Rewritten {
            // The plan and the files kept must describe the tree this invocation just wrote.
            inputs = self.inputs(&request, found.as_ref())?;
            prepared = self.prepare(&inputs)?;
        }
        let plan = &prepared.plan;
        self.consulted(&self.packages(&inputs)?, &prepared.lock)?;
        drop(preparing);
        let mut findings = plan.findings.clone();
        findings.extend(prepared.findings.iter().cloned());
        let invalid = BTreeMap::new();
        let mut refused = refusals(plan, &invalid, &mut findings);
        let root = lint_root(&inputs.repository, &plan.base);
        // A symbolic link is a path the plan places or closes, and no rule reads it: it is never followed.
        let links = inputs
            .links
            .iter()
            .filter_map(|link| relative(link, &plan.base).map(|rest| RepoPath(rest.0.into())))
            .collect::<BTreeSet<_>>();
        let corpus = listed(plan)
            .into_iter()
            .filter(|path| !links.contains(path))
            .collect::<Vec<_>>();
        let linting = tracing::info_span!("lint").entered();
        let report = self.lint(
            plan,
            LintInputs {
                root: &root,
                files: &corpus,
            },
            &mut findings,
            &mut refused,
        )?;
        let counted = self.counted(plan, &root, &report, &refused)?;
        findings.extend(counted);
        drop(linting);
        // A file whose stamp did not move since the last check keeps the digest it had: only the files that moved are read again.
        let read = prepared
            .read
            .iter()
            .map(|file| PackagedFile {
                directory: file.directory.clone(),
                file: PackageFile(file.file.0.clone()),
            })
            .collect::<Vec<_>>();
        let current = if whole {
            self.files(repository, &inputs, &read, &kept.files)
        } else {
            Vec::new()
        };
        let known = self.known_digest(plan);
        let reuse = reusable(&kept, &current, &known);
        let judging = tracing::info_span!("judge").entered();
        let judged = self.judge(repository, &inputs, plan, report, findings, &reuse)?;
        drop(judging);
        if whole {
            let checked = checked_of(&judged, current, known);
            // A store that cannot be written costs the next run its answer, never this run its answer: the failure is
            // said on stderr.
            if let Err(error) = self.store.keep_checked(&checked) {
                tracing::warn!(message = %error);
            }
        }
        Ok(CheckReport {
            report: judged,
            repository: inputs.repository,
            lock: lock_outcome,
        })
    }
}
