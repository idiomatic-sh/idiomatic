// ignore: idiomatic.arch/file-length - validate is one flow over eight ports: it reads every version in use of every package's definitions and the store's proofs, writes each scenario's files to a project held in memory and checks it there, and keeps the proofs; what the steps decide (a scenario's workspace, rules and params, what a scenario found against what it expects, a version's subject and entry heads, and, for a scenario's project, placement, planning, governance, linting, rendering and reporting) is in the domain concepts it calls, so what is left is the order of the port calls and the state one call hands the next, which no domain concept can hold.
//! The `validate` use case: every version in use of every definition of the workspace's packages proved, each proof kept
//! under the digest of the version's files and of the versions it uses, so an unchanged version is a lookup.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use rayon::prelude::*;

use super::errors::ValidateError;
use super::interface::ValidateUseCase;
use super::service::{
    manifest_error, owned_label, owned_release, place_of, problem, unchecked, unread, versions,
};
use super::types::{
    AskedProof, Digesting, FixturePlace, FixturePlacing, Materialized, ProjectJudged, Proved,
    Proving, ValidateReport, ValidateRequest, Version, VersionFile,
};
use crate::domain::applications::types::{AnswerCell, CellValue, LintAnswer, LintUnknown};
use crate::domain::definitions::constants::{LOCK_FILE, WORKSPACE_FILE};
use crate::domain::definitions::types::ErrorFile;
use crate::domain::definitions::types::LocalText;
use crate::domain::definitions::types::QueryText;
use crate::domain::definitions::types::{
    Definition, LoadedWorkspace, Role, RuleFile, SpecFile, TemplateFile, WorkspaceSection,
};
use crate::domain::files::types::{FileText, MachinePath};
use crate::domain::loading::service::{begin, definition_read, finish};
use crate::domain::loading::types::{Parsed, ProjectInputs};
use crate::domain::packages::constants::PACKAGE_FILE;
use crate::domain::packages::service::{kind_directory, kind_file, kind_word};
use crate::domain::packages::types::{
    DefinitionKind, Ownership, PackageName, PackageRoot, Reach, Release, ResolvedPackage,
    SourcePath,
};
use crate::domain::paths::service::join;
use crate::domain::paths::types::PathText;
use crate::domain::paths::types::RepoPath;
use crate::domain::placement::service::place;
use crate::domain::planning::service::plan;
use crate::domain::planning::types::ProjectPlan;
use crate::domain::proofs::service::proof;
use crate::domain::proofs::types::{DefinitionLabel, FoundOwn, Grounds, Problem, Proof, Running};
use crate::domain::proofs::types::{
    DefinitionReading, ProofFile, ProofPath, ProvenVersion, WrittenFile,
};
use crate::domain::scenario_entries::service::{
    root_entry, rule_entry, spec_entry, template_entry,
};
use crate::domain::scenario_files::types::{
    OwnWorkspace, ScenarioEntry, ScenarioKeys, ScenarioRoot, Wrapper,
};
use crate::domain::scenario_proving::service::{
    covered, described, entry_head, explained, judged_rule, judged_spec, judged_template,
    shared_releases, wrapper_file, written_home, written_workspaces,
};
use crate::domain::scenario_proving::types::ProjectScenario;
use crate::domain::selectors::types::{PathTest, PatternFailure, Spelling};
use crate::outbound::analysis::interface::AnalysisPort;
use crate::outbound::analysis::types::{
    AnalysisReport, AnalysisRequest, Corpus, Hosted, QueryVerdict,
};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestParts;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::DefinitionText;
use crate::outbound::matching::interface::MatchingPort;
use crate::outbound::packages::errors::PackagesError;
use crate::outbound::packages::interface::PackagesPort;
use crate::outbound::packages::types::PackageFile;
use crate::outbound::repository::errors::RepositoryError;
use crate::outbound::repository::interface::{Repository, RepositoryPort};
use crate::outbound::repository::types::Presence;
use crate::outbound::templates::interface::TemplatesPort;
use crate::outbound::validate_store::interface::ValidateStorePort;

/// `validate`, over the ports the edge injects.
pub struct ValidateUseCaseImpl {
    /// Runs the invocation's work.
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    /// Reads and writes the workspace's files.
    repository: Arc<dyn RepositoryPort>,
    /// Where each scenario's project is written, checked and discarded.
    fixtures: Arc<dyn RepositoryPort>,
    /// The store's directory, beneath which each scenario's project is given a root of its own.
    settings: crate::domain::settings::types::Store,
    /// How many projects it has made, so each root is a new one.
    made: std::sync::atomic::AtomicU64,
    /// Shapes and writes the report the invocation answers.
    reporting: Arc<crate::application::reporting::service::ReportingService>,
    /// Parses and edits the manifests and the lock.
    manifest: Arc<dyn ManifestPort>,
    /// Locates the packages and reads their files.
    packages: Arc<dyn PackagesPort>,
    /// Answers the analysis the definitions ask for.
    analysis: Arc<dyn AnalysisPort>,
    /// Keeps the versions that held: the project's store.
    store: Arc<dyn ValidateStorePort>,
    /// Digests the files' contents.
    digest: Arc<dyn DigestPort>,
    /// Compiles the selectors' spellings into path tests.
    matching: Arc<dyn MatchingPort>,
    /// Renders the templates' contents.
    templates: Arc<dyn TemplatesPort>,
    /// Every definition the scenarios' projects of this run read, by its file, as a project reads it: the projects of
    /// many scenarios read the same definitions, so each is read and parsed once a run.
    parsed: super::types::Parses,
    /// Every query this use case had judged, by its text, with ryo's verdict: one verdict a text.
    verdicts: super::types::Verdicts,
    /// Every package manifest this run reads, the workspace's and its scenarios' projects', by the package's directory.
    manifests: super::types::Manifests,
    /// Every definition's versions this run lists, by the package's directory and the definition's.
    offers: super::types::Offers,
    /// The wrapper packages made this run, by what they hold: the scenarios that place one definition a spec owns with
    /// one entry share one wrapper, which the packages port holds in memory.
    wrappers: super::types::Wrappers,
}

impl std::fmt::Debug for ValidateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// The ports `validate` reads and writes through.
pub struct ValidatePorts {
    /// The invocation worker scope.
    pub scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    /// The repository filesystem.
    pub repository: Arc<dyn RepositoryPort>,
    /// Where each scenario's project is written: the reporting service reads the same.
    pub fixtures: Arc<dyn RepositoryPort>,
    /// Where the project's store is: each scenario's project is rooted beneath its directory.
    pub settings: crate::domain::settings::types::Store,
    /// Shared reporting operation assembled at the edge.
    pub reporting: Arc<crate::application::reporting::service::ReportingService>,
    /// The definitions' text.
    pub manifest: Arc<dyn ManifestPort>,
    /// The packages.
    pub packages: Arc<dyn PackagesPort>,
    /// ryo.
    pub analysis: Arc<dyn AnalysisPort>,
    /// The project's store, the versions that held kept in it.
    pub store: Arc<dyn ValidateStorePort>,
    /// The digests.
    pub digest: Arc<dyn DigestPort>,
    /// The pattern compiler.
    pub matching: Arc<dyn MatchingPort>,
    /// The template renderer.
    pub templates: Arc<dyn TemplatesPort>,
}

impl std::fmt::Debug for ValidatePorts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ValidateUseCaseImpl {
    /// The caches a run keeps, for the tests that prove them bounded.
    #[cfg(test)]
    pub(super) fn caches(&self) -> super::types::Caches<'_> {
        super::types::Caches {
            parses: &self.parsed,
            manifests: &self.manifests,
            offers: &self.offers,
            verdicts: &self.verdicts,
            wrappers: &self.wrappers,
        }
    }

    /// The role of the `tastemaker.toml` at `path`, root or local, or why it does not read, as a problem of the fixture.
    /// Only a root file below the workspace makes a nested workspace whose tree it does not govern.
    fn role_of(&self, repository: &dyn Repository, path: &RepoPath) -> Result<Role, Problem> {
        let text = repository.read(path).map_err(unchecked)?;
        self.manifest
            .workspace_file(&DefinitionText(text.0))
            .map(|read| read.role)
            .map_err(unread(path))
    }

    /// `validate` over the ports the edge injects.
    #[must_use]
    pub fn new(ports: ValidatePorts) -> Self {
        Self {
            scheduler: ports.scheduler,
            repository: ports.repository,
            fixtures: ports.fixtures,
            settings: ports.settings,
            made: std::sync::atomic::AtomicU64::default(),
            reporting: ports.reporting,
            manifest: ports.manifest,
            packages: ports.packages,
            analysis: ports.analysis,
            store: ports.store,
            digest: ports.digest,
            matching: ports.matching,
            templates: ports.templates,
            parsed: super::types::Parses::default(),
            verdicts: super::types::Verdicts::default(),
            manifests: super::types::Manifests::default(),
            offers: super::types::Offers::default(),
            wrappers: super::types::Wrappers::default(),
        }
    }

    /// ryo's verdict on `query`: the one it gave before for the same text, else asked now.
    fn verdict(&self, query: &QueryText) -> Result<QueryVerdict, ValidateError> {
        if let Some(known) = self
            .verdicts
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(query)
        {
            return Ok(known.clone());
        }
        let verdict = self.analysis.validate(query)?;
        let mut kept = self
            .verdicts
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if kept.len() < super::constants::VERDICTS {
            drop(kept.insert(query.clone(), verdict.clone()));
        }
        Ok(verdict)
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

    /// The workspace's packages, direct and transitive, breadth first.
    fn packages(
        &self,
        section: &WorkspaceSection,
        directory: &PackageRoot,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, ValidateError> {
        let mut queue = section
            .dependencies
            .iter()
            .map(|dependency| {
                (
                    dependency.clone(),
                    directory.clone(),
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
            let manifest = self
                .package_read(&located.directory)
                .map_err(|why| match why {
                    super::types::Unread::File(reason) => ValidateError::from(reason),
                    super::types::Unread::Text(reason) => {
                        manifest_error(ErrorFile(PACKAGE_FILE))(reason)
                    }
                })?;
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

    /// A package file read, with the path it was read at.
    fn read(
        &self,
        directory: &PackageRoot,
        file: PackageFile,
    ) -> Result<VersionFile, ValidateError> {
        let text = self.packages.read(directory, &file)?;
        Ok(VersionFile { path: file, text })
    }

    /// What a proof's inputs take from the definition file `file` (a `rule.toml`, `spec.toml` or `template.toml`), read
    /// and parsed now; none for any other file or one that does not read.
    fn reading(&self, file: &ProofFile) -> DefinitionReading {
        let name = file.path.0.rsplit('/').next().unwrap_or_default();
        let Some(kind) = [
            DefinitionKind::Rule,
            DefinitionKind::Spec,
            DefinitionKind::Template,
        ]
        .into_iter()
        .find(|kind| name == kind_file(*kind).0) else {
            return DefinitionReading::default();
        };
        self.packages
            .read(&file.directory, &PackageFile(file.path.0.clone()))
            .ok() // ignore: idiomatic.arch/no-ok-on-result - a definition whose file does not read composes nothing here: validating that definition reports why
            .and_then(|text| self.manifest.definition(kind, &DefinitionText(text.0)).ok()) // ignore: idiomatic.arch/no-ok-on-result - a definition that does not parse composes nothing here: validating that definition reports why
            .map(|definition| DefinitionReading {
                composed: match &definition {
                    Definition::Spec(spec) => shared_releases(&spec.composition),
                    Definition::Rule(_) | Definition::Template(_) => Vec::new(),
                },
                written: written_workspaces(&definition)
                    .into_iter()
                    .map(|written| WrittenFile {
                        path: written.path.clone(),
                        text: FileText(written.text.0.into()),
                    })
                    .collect(),
            })
            .unwrap_or_default()
    }

    /// The files of the package at `directory`: as the run listed them, or listed now for a package only a scenario's
    /// own `tastemaker.toml` names.
    fn package_files(
        &self,
        directory: &PackageRoot,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
    ) -> Result<Vec<crate::domain::packages::types::ReleaseFile>, ValidateError> {
        if let Some(files) = listed.get(directory) {
            return Ok(files.clone());
        }
        Ok(self
            .packages
            .files(directory, &PackageFile(Box::default()))?
            .0
            .iter()
            .map(|file| crate::domain::packages::types::ReleaseFile(file.0.to_string()))
            .collect())
    }

    /// The digest of the files of the version at `scope`, of those `files` its package holds: each read now, by its path.
    fn content(
        &self,
        scope: &ProofFile,
        files: &[crate::domain::packages::types::ReleaseFile],
    ) -> Result<crate::domain::digests::types::Digest, ValidateError> {
        use crate::domain::proofs::service::node_parts;
        use crate::domain::proofs::types::{NodeName, NodePart};
        let prefix = format!("{}/", scope.path.0);
        let mut read = files
            .iter()
            .filter(|file| file.0.starts_with(&prefix))
            .map(|file| {
                let text = self
                    .packages
                    .read(&scope.directory, &PackageFile(file.0.clone().into()))?;
                Ok((
                    file.0.as_str(),
                    self.digest.digest(DigestParts(&[text.0.as_bytes()])),
                ))
            })
            .collect::<Result<Vec<_>, ValidateError>>()?;
        read.sort_by(|left, right| left.0.cmp(right.0));
        Ok(self.digest.digest(DigestParts(
            &node_parts(read.iter().map(|(path, digest)| NodePart {
                name: NodeName(path),
                digest,
            }))
            .0,
        )))
    }

    /// The digest of the version `release` as its files are now: the digest of its directory's files, and of the files
    /// of each version it uses, in the order `uses` names them. Noted in `digesting`.
    fn digest_of(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<crate::domain::digests::types::Digest, ValidateError> {
        use crate::domain::proofs::service::{node_parts, use_text};
        use crate::domain::proofs::types::{NodeName, NodePart, UseText};
        let own = self.own_digest(release, packages, listed, digesting)?;
        let own_name = UseText::default();
        let uses = digesting.uses.get(release).cloned().unwrap_or_default();
        let mut parts = Vec::with_capacity(uses.len());
        for used in &uses {
            parts.push((
                use_text(used),
                self.own_digest(used, packages, listed, digesting)?,
            ));
        }
        let digest = self.digest.digest(DigestParts(
            &node_parts(
                std::iter::once((&own_name, &own))
                    .chain(parts.iter().map(|(used, digest)| (used, digest)))
                    .map(|(name, digest)| NodePart {
                        name: NodeName(&name.0),
                        digest,
                    }),
            )
            .0,
        ));
        drop(digesting.done.insert(release.clone(), digest.clone()));
        Ok(digest)
    }

    /// The digest of the files of the version `release` alone, read once a run.
    fn own_digest(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<crate::domain::digests::types::Digest, ValidateError> {
        let Some(scope) = place_of(release, packages, &digesting.located) else {
            return Ok(crate::domain::digests::types::Digest::default());
        };
        if let Some(content) = digesting.contents.get(&scope) {
            return Ok(content.clone());
        }
        let files = self.package_files(&scope.directory, listed)?;
        let content = self.content(&scope, &files)?;
        drop(digesting.contents.insert(scope, content.clone()));
        Ok(content)
    }

    /// The versions `release` uses as its definitions say now, and those they use in turn, each placed, in order: read
    /// and parsed, a version's definitions once a run.
    fn resolve_uses(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<(), ValidateError> {
        let mut reached = BTreeSet::new();
        let mut pending = vec![release.clone()];
        while let Some(current) = pending.pop() {
            if !reached.insert(current.clone()) {
                continue;
            }
            if !digesting.direct.contains_key(&current) {
                let named = self.direct_of(&current, packages, listed, &mut digesting.located)?;
                drop(digesting.direct.insert(current.clone(), named));
            }
            pending.extend(
                digesting
                    .direct
                    .get(&current)
                    .into_iter()
                    .flatten()
                    .cloned(),
            );
        }
        let _: bool = reached.remove(release);
        let _: bool = digesting.resolved.insert(release.clone());
        drop(
            digesting
                .uses
                .insert(release.clone(), reached.into_iter().collect()),
        );
        Ok(())
    }

    /// The definitions of every version of `unstamped` whose uses may be read again, read at once: the walk of the uses
    /// finds them read.
    fn read_definitions(
        &self,
        unstamped: &[&Version],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<(), ValidateError> {
        let located = digesting.located.clone();
        let read = unstamped
            .par_iter()
            .filter(|version| !digesting.direct.contains_key(&version.release))
            .map(|version| {
                let mut found = located.clone();
                let named = self.direct_of(&version.release, packages, listed, &mut found)?;
                Ok((version.release.clone(), named, found))
            })
            .collect::<Result<Vec<_>, ValidateError>>()?;
        for (release, named, found) in read {
            drop(digesting.direct.insert(release, named));
            digesting.located.extend(found);
        }
        Ok(())
    }

    /// The versions the definitions of `release` name, read now; the packages a scenario's own `tastemaker.toml` names
    /// noted in `located`.
    fn direct_of(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        located: &mut std::collections::HashMap<PackageName, PackageRoot>,
    ) -> Result<Vec<Release>, ValidateError> {
        let Some(scope) = place_of(release, packages, located) else {
            return Ok(Vec::new());
        };
        let files = self.package_files(&scope.directory, listed)?;
        let prefix = format!("{}/", scope.path.0);
        let paths = files
            .iter()
            .filter(|file| file.0.starts_with(&prefix))
            .map(|file| ProofPath(file.0.clone().into()))
            .collect::<Vec<_>>();
        self.references(&scope, &paths, packages, located)
    }

    /// Whether every version a kept row `uses` names is still where it was, and each a scenario's own `tastemaker.toml`
    /// resolved still the latest its definition offers: such a use reads the latest, so a newer one resolves it again.
    fn uses_hold(
        &self,
        uses: &[Release],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<Grounds, ValidateError> {
        use crate::domain::packages::service::{held_versions, latest};
        for used in uses {
            let Some(scope) = place_of(used, packages, &digesting.located) else {
                return Ok(Grounds::Moved);
            };
            // A spec's composition pins the version it uses; only a use a scenario's own `tastemaker.toml` resolved, under
            // a name that says where its package is, reads the latest.
            if !used.package.0.contains('@') {
                continue;
            }
            if !digesting.offered.contains_key(&scope.directory) {
                let files = self.package_files(&scope.directory, listed)?;
                drop(
                    digesting
                        .offered
                        .insert(scope.directory.clone(), held_versions(&files)),
                );
            }
            let offered = digesting
                .offered
                .get(&scope.directory)
                .and_then(|held| {
                    held.get(&crate::domain::packages::types::DefinitionKey {
                        kind: used.kind,
                        name: used.name.clone(),
                    })
                })
                .map(Vec::as_slice)
                .unwrap_or_default();
            if !offered.contains(&used.version)
                || latest(&used.version, offered.iter()) != used.version
            {
                return Ok(Grounds::Moved);
            }
        }
        Ok(Grounds::Holds)
    }

    /// The stamp of the version at `scope`, of those `files` its package holds: the size of its files, all told, and the
    /// latest time any of them, or a directory of the version, was modified; a file added, removed, renamed or written
    /// moves it.
    fn summary(
        &self,
        scope: &ProofFile,
        files: &[crate::domain::packages::types::ReleaseFile],
    ) -> crate::domain::files::types::Stamp {
        use crate::domain::proofs::service::{summary, version_paths};
        use crate::outbound::packages::types::PackagedFile;
        let paths = version_paths(
            files.iter().map(|file| PathText(&file.0)),
            PathText(&scope.path.0),
        );
        let asked = paths
            .files
            .iter()
            .chain(&paths.directories)
            .map(|path| PackagedFile {
                directory: scope.directory.clone(),
                file: PackageFile(path.0.into()),
            })
            .collect::<Vec<_>>();
        let stamps = self.packages.stamps(&asked);
        let (of_files, of_directories) = stamps.0.split_at(paths.files.len().min(stamps.0.len()));
        summary(of_files, of_directories)
    }

    /// The stamp of the version `release` and the versions it `uses`, all told: while it does not move, neither does any
    /// file the version's digest is made from.
    fn combined(
        &self,
        release: &Release,
        uses: &[Release],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<crate::domain::files::types::Stamp, ValidateError> {
        let mut stamps = Vec::with_capacity(uses.len() + 1);
        for each in std::iter::once(release).chain(uses) {
            stamps.push(self.stamp_of(each, packages, listed, digesting)?);
        }
        Ok(crate::domain::proofs::service::summary(&stamps, &[]))
    }

    /// The stamp of the version `release`, taken once a run; the default stamp when it is not found.
    fn stamp_of(
        &self,
        release: &Release,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        digesting: &mut Digesting,
    ) -> Result<crate::domain::files::types::Stamp, ValidateError> {
        if let Some(stamp) = digesting.stamps.get(release) {
            return Ok(*stamp);
        }
        let stamp = match place_of(release, packages, &digesting.located) {
            Some(scope) => self.summary(&scope, &self.package_files(&scope.directory, listed)?),
            None => crate::domain::files::types::Stamp::default(),
        };
        let _: Option<crate::domain::files::types::Stamp> =
            digesting.stamps.insert(release.clone(), stamp);
        Ok(stamp)
    }

    /// The versions of `wanted` whose kept row's stamp, its uses' with it, moved, or that have no row; each other keeps
    /// its row's digest and uses in `digesting`, its files unread.
    fn unstamped<'a>(
        &self,
        wanted: &[&'a Version],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        kept: &BTreeMap<Release, ProvenVersion>,
        digesting: &mut Digesting,
    ) -> Result<Vec<&'a Version>, ValidateError> {
        let mut unstamped = Vec::new();
        for version in wanted {
            let still = match kept.get(&version.release) {
                Some(row) => {
                    self.combined(&version.release, &row.uses, packages, listed, digesting)?
                        == row.stamp
                        && self.uses_hold(&row.uses, packages, listed, digesting)? == Grounds::Holds
                }
                None => false,
            };
            match kept.get(&version.release) {
                Some(row) if still => {
                    drop(
                        digesting
                            .uses
                            .insert(version.release.clone(), row.uses.clone()),
                    );
                    drop(
                        digesting
                            .done
                            .insert(version.release.clone(), row.digest.clone()),
                    );
                }
                _ => unstamped.push(*version),
            }
        }
        Ok(unstamped)
    }

    /// The digest of each version a row is kept for or this run asks, `asked` says which, as its files are now, and the
    /// versions it uses. A version kept whose stamp, and whose uses' stamps, did not move keeps its digest and uses, its
    /// files unread; one kept with uses that still hold takes them unread; any other, and one whose digest moved, is
    /// read and parsed again. A version neither kept nor asked is not read: no row is checked against it.
    fn digests(
        &self,
        versions: &[Version],
        only: &[super::types::VersionFilter],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        kept: &BTreeMap<Release, ProvenVersion>,
    ) -> Result<Digesting, ValidateError> {
        let _keying = tracing::info_span!("digests").entered();
        let mut digesting = Digesting::default();
        let wanted = versions
            .iter()
            .filter(|version| {
                kept.contains_key(&version.release)
                    || only.is_empty()
                    || only
                        .iter()
                        .any(|filter| version.subject.id.0.contains(&*filter.0))
            })
            .collect::<Vec<_>>();
        // Each version's stamp, taken at once.
        let stamps = wanted
            .par_iter()
            .map(|version| {
                let scope = ProofFile {
                    directory: version.package.clone(),
                    path: ProofPath(version.subject.directory.0.clone()),
                };
                let files = listed
                    .get(&version.package)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                (version.release.clone(), self.summary(&scope, files))
            })
            .collect::<Vec<_>>();
        digesting.stamps.extend(stamps);
        let unstamped = self.unstamped(&wanted, packages, listed, kept, &mut digesting)?;
        // Each other version's own files, read at once.
        let contents = unstamped
            .par_iter()
            .map(|version| {
                let scope = ProofFile {
                    directory: version.package.clone(),
                    path: ProofPath(version.subject.directory.0.clone()),
                };
                let files = listed
                    .get(&version.package)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let content = self.content(&scope, files)?;
                Ok((scope, content))
            })
            .collect::<Result<Vec<_>, ValidateError>>()?;
        digesting.contents.extend(contents);
        self.read_definitions(&unstamped, packages, listed, &mut digesting)?;
        for version in &unstamped {
            match kept.get(&version.release) {
                Some(row)
                    if self.uses_hold(&row.uses, packages, listed, &mut digesting)?
                        == Grounds::Holds =>
                {
                    drop(
                        digesting
                            .uses
                            .insert(version.release.clone(), row.uses.clone()),
                    );
                }
                _ => self.resolve_uses(&version.release, packages, listed, &mut digesting)?,
            }
        }
        // A version whose digest moved may use others now: its definitions are read again, and every digest made again.
        let mut moved = false;
        for version in &unstamped {
            let digest = self.digest_of(&version.release, packages, listed, &mut digesting)?;
            if !digesting.resolved.contains(&version.release)
                && kept
                    .get(&version.release)
                    .is_some_and(|row| row.digest != digest)
            {
                drop(digesting.uses.remove(&version.release));
                self.resolve_uses(&version.release, packages, listed, &mut digesting)?;
                moved = true;
            }
        }
        if moved {
            for version in &unstamped {
                drop(self.digest_of(&version.release, packages, listed, &mut digesting)?);
            }
        }
        // Each version's stamp, its uses' with it, as a row keeps it.
        for version in &wanted {
            let uses = digesting
                .uses
                .get(&version.release)
                .cloned()
                .unwrap_or_default();
            let stamp = self.combined(&version.release, &uses, packages, listed, &mut digesting)?;
            let _: Option<crate::domain::files::types::Stamp> =
                digesting.kept.insert(version.release.clone(), stamp);
        }
        Ok(digesting)
    }

    /// The versions the version at `scope`, whose files are `files`, uses: those its spec's composition names, and
    /// those its scenarios' own `tastemaker.toml` files name, each where its package is.
    fn references(
        &self,
        scope: &ProofFile,
        files: &[ProofPath],
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        located: &mut std::collections::HashMap<PackageName, PackageRoot>,
    ) -> Result<Vec<Release>, ValidateError> {
        let spec = format!("{}/{}", scope.path.0, kind_file(DefinitionKind::Spec).0);
        let mut releases = Vec::new();
        for path in files {
            let file = ProofFile {
                directory: scope.directory.clone(),
                path: path.clone(),
            };
            let reading = self.reading(&file);
            if *path.0 == *spec {
                releases.extend(reading.composed);
            }
            releases.extend(self.scenario_references(&file, &reading.written, located)?);
        }
        let mut references = releases
            .into_iter()
            .filter(|release| place_of(release, packages, located).is_some())
            .map(|release| Release {
                ownership: Ownership::Shared,
                ..release
            })
            .collect::<Vec<_>>();
        references.sort();
        references.dedup();
        Ok(references)
    }

    /// The releases the `tastemaker.toml` files `written` in the scenarios of the definition file `file` name from other
    /// packages: a proof reads them too, so a change to one proves the definition again. The packages those files
    /// depend on, which the project's own dependencies may not reach, are noted in `located` with where they are.
    fn scenario_references(
        &self,
        file: &ProofFile,
        written: &[WrittenFile],
        located: &mut std::collections::HashMap<PackageName, PackageRoot>,
    ) -> Result<Vec<Release>, PackagesError> {
        let mut workspaces = Vec::new();
        for each in written {
            let here = written_home(&file.directory, &file.path, &each.path);
            let read = self
                .manifest
                .rebased(&each.text, &here)
                .and_then(|rebased| self.manifest.workspace_file(&DefinitionText(rebased.0)));
            // A scenario's `tastemaker.toml` that does not read (a scenario of a refusal) names no package.
            match read {
                Ok(workspace) => workspaces.push(workspace),
                Err(
                    ManifestError::Shape { .. }
                    | ManifestError::LockVersion { .. }
                    | ManifestError::Needs { .. },
                ) => {}
            }
        }
        let mut releases = Vec::new();
        for workspace in &workspaces {
            let resolved = match &workspace.role {
                Role::Root(section) => self
                    .packages(section, &PackageRoot(Box::default()))
                    .unwrap_or_default(),
                Role::Local => BTreeMap::new(),
            };
            // A release the scenario's own file names is read from where that file finds the package, under a name
            // that says where: two scenarios may place different packages under one name.
            for release in shared_releases(&workspace.composition) {
                releases.push(match resolved.get(&release.package) {
                    Some(package) => {
                        let key = PackageName(
                            format!("{}@{}", release.package.0, package.directory.0).into(),
                        );
                        drop(located.insert(key.clone(), package.directory.clone()));
                        // A scenario's tastemaker.toml pins no version: its check reads the latest offered.
                        let offered = self.offered(
                            &package.directory,
                            PackageFile(
                                format!("{}/{}", kind_directory(release.kind), release.name.0)
                                    .into(),
                            ),
                        )?;
                        Release {
                            package: key,
                            version: crate::domain::packages::service::latest(
                                &release.version,
                                offered.iter(),
                            ),
                            ..release
                        }
                    }
                    None => release,
                });
            }
            // The packages those packages name are found by their plain names.
            for (named, package) in resolved {
                let _: &mut PackageRoot = located.entry(named).or_insert(package.directory);
            }
        }
        Ok(releases)
    }

    /// Writes a scenario's project into the scratch repository `repository`, each `tastemaker.toml` rebased onto the
    /// directory of the asset the scenario is written in, and every file written into `written`: whether the project brings
    /// its own.
    fn project(
        &self,
        package: &PackageRoot,
        repository: &dyn Repository,
        files: &BTreeMap<RepoPath, Materialized>,
        written: &mut BTreeMap<RepoPath, FileText>,
    ) -> Result<OwnWorkspace, Problem> {
        use crate::domain::scenario_files::constants::{GIT_HEAD, GIT_HEAD_TEXT, WORKSPACE};
        let mut write = |path: &RepoPath, text: FileText| {
            repository.write(path, &text).map_err(problem)?;
            drop(written.insert(path.clone(), text));
            Ok::<(), Problem>(())
        };
        write(&RepoPath(GIT_HEAD.into()), FileText(GIT_HEAD_TEXT.into()))?;
        let mut own = OwnWorkspace::Absent;
        for (path, Materialized { text, origin }) in files {
            let text = if path.0.rsplit('/').next() == Some(WORKSPACE) {
                own = OwnWorkspace::Written;
                let parent = crate::domain::paths::service::parent(path).unwrap_or_default();
                let here = std::path::Path::new(&*package.0)
                    .join(&*origin.0)
                    .join(&*parent.0)
                    .display()
                    .to_string();
                self.manifest
                    .rebased(text, &RepoPath(here.into()))
                    .map_err(|reason| problem(format_args!("`{}`: {reason}", path.0)))?
                    .0
            } else {
                text.0.clone()
            };
            write(path, FileText(text))?;
        }
        Ok(own)
    }

    /// The `tastemaker.toml` placing what a fixture without its own proves: the definition by an entry with the
    /// scenario's `keys`, or, for one a spec owns, a wrapper package the packages port holds in memory, holding only the
    /// spec's version with only the definition, applied at the root.
    fn placed(
        &self,
        version: &Version,
        place: &FixturePlace,
        keys: &ScenarioKeys,
    ) -> Result<FileText, Problem> {
        let release = &version.release;
        match place {
            FixturePlace::Shared { placed } => Ok(FileText(
                self.manifest
                    .scenario_root(&ScenarioRoot {
                        path: &version.package,
                        placed,
                        keys,
                    })
                    .0,
            )),
            FixturePlace::Owned {
                kind,
                name,
                directory,
            } => {
                let owner = FileText(
                    self.manifest
                        .scenario_wrapper(&Wrapper {
                            kind: *kind,
                            name,
                            spec: &release.name,
                            keys,
                        })
                        .0,
                );
                let made = self.wrapper(version, directory, owner)?;
                Ok(FileText(
                    self.manifest
                        .scenario_root(&ScenarioRoot {
                            path: &made,
                            placed: &entry_head(release),
                            keys: &root_entry(),
                        })
                        .0,
                ))
            }
        }
    }

    /// The directory of the wrapper package holding only `version`'s spec, whose definition file is `owner`, with only
    /// the definition at `directory`: made by the first scenario that asks for it, and shared by every other.
    fn wrapper(
        &self,
        version: &Version,
        directory: &PackageFile,
        owner: FileText,
    ) -> Result<PackageRoot, Problem> {
        let once = {
            let mut wrappers = self
                .wrappers
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let key = super::types::WrapperKey {
                package: version.package.clone(),
                directory: directory.clone(),
                owner: owner.clone(),
            };
            // Past its bound a wrapper is made for the one scenario that asks, and not kept.
            if wrappers.len() < super::constants::WRAPPERS || wrappers.contains_key(&key) {
                Arc::clone(wrappers.entry(key).or_default())
            } else {
                Arc::default()
            }
        };
        once.0
            .get_or_init(|| self.wrapped(version, directory, owner))
            .clone()
    }

    /// Makes the wrapper package of `version`'s spec, whose definition file is `owner`, holding the definition at
    /// `directory`, and has the packages port hold it in memory: nothing is written to a disk.
    fn wrapped(
        &self,
        version: &Version,
        directory: &PackageFile,
        owner: FileText,
    ) -> Result<PackageRoot, Problem> {
        let mut held = crate::outbound::packages::types::HeldFiles::default();
        let mut write = |path: RepoPath, text: FileText| {
            drop(held.0.insert(PackageFile(path.0), text));
            Ok::<(), Problem>(())
        };
        let package = self
            .packages
            .read(&version.package, &PackageFile(PACKAGE_FILE.into()))
            .map_err(problem)?;
        let package = self
            .manifest
            .rebased(&package, &RepoPath(version.package.0.clone()))
            .map_err(problem)?;
        write(RepoPath(PACKAGE_FILE.into()), package)?;
        write(
            wrapper_file(&version.subject.directory, version.release.kind),
            owner,
        )?;
        for file in self
            .packages
            .files(&version.package, directory)
            .map_err(problem)?
        {
            let text = self
                .packages
                .read(
                    &version.package,
                    &PackageFile(format!("{}/{}", directory.0, file.0).into()),
                )
                .map_err(problem)?;
            write(RepoPath(format!("{}/{}", directory.0, file.0).into()), text)?;
        }
        Ok(self.packages.hold(held))
    }

    /// Proves one version.
    fn prove(&self, version: &Version) -> Result<Proof, ValidateError> {
        let files = version
            .files
            .iter()
            .map(|path| self.read(&version.package, path.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        let Some(VersionFile { path: file, text }) = files.first() else {
            return Ok(Proof::Failed(vec![Problem(
                crate::domain::proofs::constants::NO_DEFINITION_FILE.into(),
            )]));
        };
        let text = DefinitionText(text.0.clone());
        let refused = |reason: ManifestError| {
            Proof::Failed(vec![Problem(format!("`{}`: {reason}", file.0).into())])
        };
        let mut problems = Vec::new();
        let label = DefinitionLabel(format!(
            "{} `{}`",
            kind_word(version.release.kind).0,
            version.release.name.0
        ));
        match version.release.kind {
            DefinitionKind::Rule => {
                let rule = match self.manifest.rule(&text) {
                    Ok(rule) => rule,
                    Err(reason) => return Ok(refused(reason)),
                };
                self.rule_proof(&mut problems, &label, version, &version.release, &rule)?;
            }
            DefinitionKind::Spec => {
                let spec = match self.manifest.spec(&text) {
                    Ok(spec) => spec,
                    Err(reason) => return Ok(refused(reason)),
                };
                explained(&mut problems, &spec.description, &spec.why);
                described(&mut problems, &spec);
                for VersionFile {
                    path,
                    text: owned_text,
                } in files.iter().skip(1)
                {
                    let owned = owned_label(&label, path);
                    if path.0.ends_with(kind_file(DefinitionKind::Rule).0) {
                        match self.manifest.rule(&DefinitionText(owned_text.0.clone())) {
                            Ok(rule) => {
                                let release =
                                    owned_release(&version.release, DefinitionKind::Rule, path);
                                self.rule_proof(&mut problems, &owned, version, &release, &rule)?;
                            }
                            Err(reason) => {
                                problems.push(Problem(format!("`{}`: {reason}", path.0).into()));
                            }
                        }
                    }
                    if path.0.ends_with(kind_file(DefinitionKind::Template).0) {
                        match self
                            .manifest
                            .template(&DefinitionText(owned_text.0.clone()))
                        {
                            Ok(template) => {
                                let release =
                                    owned_release(&version.release, DefinitionKind::Template, path);
                                problems.extend(
                                    self.template_proof(version, &owned, &release, &template)?,
                                );
                            }
                            Err(reason) => {
                                problems.push(Problem(format!("`{}`: {reason}", path.0).into()));
                            }
                        }
                    }
                }
                covered(&mut problems, &label, &judged_spec(&spec));
                problems.extend(self.spec_cases(version, &spec)?);
            }
            DefinitionKind::Template => {
                let template = match self.manifest.template(&text) {
                    Ok(template) => template,
                    Err(reason) => return Ok(refused(reason)),
                };
                explained(&mut problems, &template.description, &template.why);
                problems.extend(self.template_proof_cases(
                    version,
                    &label,
                    &version.release,
                    &template,
                )?);
            }
        }
        Ok(proof(problems))
    }

    /// The problems of a template a spec owns: its description and why, and each of its scenarios.
    fn template_proof(
        &self,
        version: &Version,
        label: &DefinitionLabel,
        release: &Release,
        template: &TemplateFile,
    ) -> Result<Vec<Problem>, ValidateError> {
        let mut problems = Vec::new();
        explained(&mut problems, &template.description, &template.why);
        problems.extend(self.template_proof_cases(version, label, release, template)?);
        Ok(problems)
    }

    /// The problems of a template's scenarios: that they cover it, and that each holds.
    fn template_proof_cases(
        &self,
        version: &Version,
        label: &DefinitionLabel,
        release: &Release,
        template: &TemplateFile,
    ) -> Result<Vec<Problem>, ValidateError> {
        let mut problems = Vec::new();
        covered(&mut problems, label, &judged_template(template));
        problems.extend(self.template_cases(version, release, template)?);
        Ok(problems)
    }

    /// The problems of one rule: its description and why, whether its query runs, that its scenarios cover it, and each of
    /// them when the query runs.
    fn rule_proof(
        &self,
        problems: &mut Vec<Problem>,
        label: &DefinitionLabel,
        version: &Version,
        release: &Release,
        rule: &RuleFile,
    ) -> Result<(), ValidateError> {
        explained(problems, &rule.description, &rule.why);
        covered(problems, label, &judged_rule(rule));
        if self.rule_runs(problems, rule)? == Running::Runs {
            problems.extend(self.rule_cases(version, release, rule)?);
        }
        Ok(())
    }

    /// Whether a rule's query is one this ryo runs; a problem when not.
    fn rule_runs(
        &self,
        problems: &mut Vec<Problem>,
        rule: &RuleFile,
    ) -> Result<Running, ValidateError> {
        match self.verdict(&rule.query)? {
            QueryVerdict::Valid => Ok(Running::Runs),
            QueryVerdict::Invalid(reason) => {
                problems.push(Problem(
                    format!("the query is not one this ryo runs: {reason}").into(),
                ));
                Ok(Running::Refused)
            }
        }
    }

    /// The problems of a rule's scenarios, each as the check of a project of its own: the rule placed by the entry the
    /// scenario's `given` writes, or by the `tastemaker.toml` it brings.
    fn rule_cases(
        &self,
        version: &Version,
        release: &Release,
        rule: &RuleFile,
    ) -> Result<Vec<Problem>, ValidateError> {
        // The id its findings carry: the package, the spec that owns it when one does, and its name.
        let rule_id = match &release.ownership {
            Ownership::Shared => format!("{}/{}", release.package.0, release.name.0),
            Ownership::Owned(_) => format!(
                "{}/{}/{}",
                release.package.0, version.release.name.0, release.name.0
            ),
        };
        let scenarios = rule
            .scenarios
            .iter()
            .map(|scenario| ProjectScenario {
                id: &scenario.id,
                own: Some(FoundOwn(&rule_id)),
                entry: ScenarioEntry::Keys(rule_entry(scenario)),
                files: &scenario.files,
                only: &scenario.only,
                expected: &scenario.findings,
            })
            .collect::<Vec<_>>();
        let directory = match &release.ownership {
            Ownership::Shared => version.subject.directory.0.to_string(),
            Ownership::Owned(_) => format!(
                "{}/{}/{}",
                version.subject.directory.0,
                kind_directory(DefinitionKind::Rule).0,
                release.name.0
            ),
        };
        self.project_cases(
            version,
            &|| match &release.ownership {
                Ownership::Shared => FixturePlace::Shared {
                    placed: entry_head(release),
                },
                Ownership::Owned(_) => FixturePlace::Owned {
                    kind: DefinitionKind::Rule,
                    name: release.name.clone(),
                    directory: PackageFile(directory.clone().into()),
                },
            },
            &PackageFile(directory.clone().into()),
            &scenarios,
        )
    }

    /// Runs the scenarios of a spec or a template in parallel, each as the check of a project of its own: the scenario's
    /// files written to a scratch repository, the definition placed there by the `tastemaker.toml` the scenario brings or
    /// one written for it, the project checked, and the findings compared with the ones the scenario expects.
    fn project_cases(
        &self,
        version: &Version,
        place: &(dyn Fn() -> FixturePlace + Sync),
        origin: &PackageFile,
        scenarios: &[ProjectScenario<'_>],
    ) -> Result<Vec<Problem>, ValidateError> {
        let completed = scenarios
            .par_iter()
            .map(|scenario| {
                self.project_case(
                    &version.package,
                    origin,
                    Some(&FixturePlacing {
                        version,
                        place: place(),
                    }),
                    scenario,
                )
            })
            .collect::<Vec<_>>();
        // Every checked scenario's findings and expectations are hosted for one verdict call over them all.
        let mut rows = crate::domain::scenario_judging::types::VerdictRows::default();
        let mut judged = Vec::with_capacity(scenarios.len());
        for (number, (scenario, each)) in scenarios.iter().zip(completed).enumerate() {
            match each? {
                ProjectJudged::Checked(report) => {
                    crate::domain::scenario_judging::service::host(
                        crate::domain::scenario_judging::types::ScenarioNumber(
                            u32::try_from(number).unwrap_or(u32::MAX),
                        ),
                        scenario.own,
                        scenario.expected,
                        scenario.only,
                        &report.findings,
                        &report.evaluated,
                        &mut rows,
                    );
                    judged.push(None);
                }
                ProjectJudged::Unchecked(problem) => judged.push(Some(problem)),
            }
        }
        let verdicts = self.verdicts(&version.package, rows)?;
        let mut problems = Vec::new();
        for (number, (scenario, unchecked)) in scenarios.iter().zip(judged).enumerate() {
            if let Some(problem) = unchecked {
                problems.push(problem);
                continue;
            }
            let number = crate::domain::scenario_judging::types::ScenarioNumber(
                u32::try_from(number).unwrap_or(u32::MAX),
            );
            let own = verdicts
                .iter()
                .filter(|verdict| {
                    crate::domain::scenario_judging::service::scenario_of(verdict) == number
                })
                .collect::<Vec<_>>();
            problems.extend(crate::domain::scenario_judging::service::problems(
                scenario.id,
                &own,
            ));
        }
        Ok(problems)
    }

    /// The verdict of the scenarios `rows` hosts: one lint call over the verdict queries, its corpus empty and every
    /// relation hosted, and the rows it answers; none when no scenario expects, finds, names or proves anything.
    fn verdicts(
        &self,
        package: &PackageRoot,
        rows: crate::domain::scenario_judging::types::VerdictRows,
    ) -> Result<Vec<crate::domain::scenario_judging::types::ScenarioVerdict>, ValidateError> {
        if rows.expected.is_empty()
            && rows.found.is_empty()
            && rows.only.is_empty()
            && rows.own.is_empty()
        {
            return Ok(Vec::new());
        }
        let report = self.analysis.lint(AnalysisRequest {
            root: MachinePath(package.0.clone()),
            rules: crate::domain::scenario_judging::service::rules(),
            placed: Vec::new(),
            corpus: Corpus::Given(Vec::new()),
            beside: Vec::new(),
            hosted: Hosted::Verdict(rows),
        })?;
        Ok(report
            .findings
            .iter()
            .filter_map(crate::domain::scenario_judging::service::verdict_of)
            .collect())
    }

    /// One scenario as a project: its problems. `placing` is what places a project
    /// without a `tastemaker.toml` of its own: the version proven, and how its definition is placed.
    fn project_case(
        &self,
        package: &PackageRoot,
        origin: &PackageFile,
        placing: Option<&FixturePlacing<'_>>,
        scenario: &ProjectScenario<'_>,
    ) -> Result<ProjectJudged, ValidateError> {
        let number = self.made.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = MachinePath(format!("{}/project-{number}", self.settings.directory.0).into()); // ignore: idiomatic.arch/stable-ids - a scenario's project is named for no item and lives only as long as its case: the run's counter makes each root its own, and no name is kept past the run
        let held = self.fixtures.project(&root)?;
        let mut written = BTreeMap::new();
        let outcome = (|| {
            let files = scenario
                .files
                .0
                .iter()
                .map(|(path, text)| {
                    (
                        RepoPath(path.0.clone()),
                        Materialized {
                            text: FileText(text.0.clone()),
                            origin: origin.clone(),
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if self.project(package, &*held, &files, &mut written)? == OwnWorkspace::Absent {
                let (Some(placing), ScenarioEntry::Keys(entry)) = (&placing, &scenario.entry)
                else {
                    return Err(Problem(
                        crate::domain::proofs::constants::UNPLACED_SCENARIO.into(),
                    ));
                };
                let placed = self.placed(placing.version, &placing.place, entry)?;
                let at = RepoPath(crate::domain::scenario_files::constants::WORKSPACE.into());
                held.write(&at, &placed).map_err(problem)?;
                drop(written.insert(at, placed));
            }
            self.checked(
                &*held,
                &written,
                placing.map(|placing| &placing.version.release),
            )
        })();
        drop(held);
        Ok(match outcome {
            Ok(report) => ProjectJudged::Checked(report),
            Err(reason) => ProjectJudged::Unchecked(Problem(
                format!("scenario `{}`: {}", scenario.id.0, reason.0).into(),
            )),
        })
    }

    /// The scenarios of a spec, each as a project that places the spec at its `at`.
    fn spec_cases(
        &self,
        version: &Version,
        spec: &SpecFile,
    ) -> Result<Vec<Problem>, ValidateError> {
        let scenarios = spec
            .scenarios
            .iter()
            .map(|scenario| ProjectScenario {
                id: &scenario.id,
                own: None,
                entry: ScenarioEntry::Keys(spec_entry(scenario)),
                files: &scenario.files,
                only: &scenario.only,
                expected: &scenario.findings,
            })
            .collect::<Vec<_>>();
        let release = &version.release;
        self.project_cases(
            version,
            &|| FixturePlace::Shared {
                placed: entry_head(release),
            },
            &PackageFile(version.subject.directory.0.clone()),
            &scenarios,
        )
    }

    /// The scenarios of a template, the one `release` names, each as a project that places the template at its `file`.
    fn template_cases(
        &self,
        version: &Version,
        release: &Release,
        template: &TemplateFile,
    ) -> Result<Vec<Problem>, ValidateError> {
        let scenarios = template
            .scenarios
            .iter()
            .map(|scenario| ProjectScenario {
                id: &scenario.id,
                own: None,
                entry: template_entry(scenario),
                files: &scenario.files,
                only: &scenario.only,
                expected: &scenario.findings,
            })
            .collect::<Vec<_>>();
        let directory = match &release.ownership {
            Ownership::Shared => version.subject.directory.0.to_string(),
            Ownership::Owned(_) => format!(
                "{}/{}/{}",
                version.subject.directory.0,
                kind_directory(DefinitionKind::Template).0,
                release.name.0
            ),
        };
        self.project_cases(
            version,
            &|| match &release.ownership {
                Ownership::Shared => FixturePlace::Shared {
                    placed: entry_head(release),
                },
                Ownership::Owned(_) => FixturePlace::Owned {
                    kind: DefinitionKind::Template,
                    name: release.name.clone(),
                    directory: PackageFile(directory.clone().into()),
                },
            },
            &PackageFile(directory.clone().into()),
            &scenarios,
        )
    }

    // A fixture's project is checked as `check` checks a workspace: its files read, its definitions loaded, placed and
    // planned, its queries validated, its lint calls made and their answers reported. validate makes these port calls itself:
    // a use case never calls another, and the steps between the calls are the domain's, which both call.
    /// The report of checking the fixture's project `repository`, whose files are `written`: what `check --fix` reports
    /// there, the release `subject` proves pinned at its version and every other entry at the latest its package offers.
    /// Only its findings are compared, so none is fingerprinted.
    fn checked(
        &self,
        repository: &dyn Repository,
        written: &BTreeMap<RepoPath, FileText>,
        subject: Option<&Release>,
    ) -> Result<crate::domain::findings::types::Report, Problem> {
        use crate::domain::linting::service::{listed, refusals, repository_path};
        use crate::domain::linting::types::RefusalReason;
        let root = repository.root();
        // The project is its own repository, its workspace at its root: one with no root `tastemaker.toml` has none.
        if repository.exists(&RepoPath(WORKSPACE_FILE.into())) == Presence::Absent {
            return Err(unchecked(RepositoryError::NoWorkspace {
                start: root.0.to_string(),
            }));
        }
        let inputs = self.project_inputs(repository)?;
        let ProjectPlan { plan, mut findings } = self.project_plan(&inputs, subject)?;
        findings.extend(plan.findings.iter().cloned());
        let mut queries = plan
            .rules
            .iter()
            .map(|rule| &rule.query)
            .collect::<Vec<_>>();
        queries.sort();
        queries.dedup();
        let invalid = queries
            .par_iter()
            .map(|query| {
                self.verdict(query).map(|verdict| match verdict {
                    QueryVerdict::Valid => None,
                    QueryVerdict::Invalid(reason) => {
                        Some(((*query).clone(), RefusalReason(reason.0)))
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(unchecked)?
            .into_iter()
            .flatten()
            .collect();
        let mut refused = refusals(&plan, &invalid, &mut findings);
        let corpus = listed(&plan);
        let report =
            self.project_lint(root, written, &plan, &corpus, &mut findings, &mut refused)?;
        findings.extend(self.project_counted(root, &plan, &report, &refused)?);
        let answers = crate::domain::governance::types::Answers {
            answers: report
                .findings
                .into_iter()
                .map(|row| LintAnswer {
                    cells: row
                        .cells
                        .into_iter()
                        .map(|cell| AnswerCell {
                            value: match cell.value {
                                CellValue::File(path) => {
                                    CellValue::File(repository_path(&plan.base, path))
                                }
                                other @ (CellValue::Span(_) | CellValue::Text(_)) => other,
                            },
                            name: cell.name,
                        })
                        .collect(),
                    path: repository_path(&plan.base, row.path),
                    rule: row.rule,
                })
                .collect(),
            unknowns: report
                .unknowns
                .into_iter()
                .map(|missing| LintUnknown {
                    path: repository_path(&plan.base, missing.path),
                    rule: missing.rule,
                    message: missing.message,
                })
                .collect(),
            unread: report
                .unread
                .into_iter()
                .map(|path| repository_path(&plan.base, path))
                .collect(),
        };
        self.reporting
            .findings(
                repository,
                crate::domain::reporting::types::Reported {
                    plan: &plan,
                    answers,
                    findings,
                    within: &[],
                    reuse: &crate::domain::reporting::types::Reuse::default(),
                },
            )
            .map_err(|error| Problem(error.to_string().into()))
    }

    /// The project's lint call over `plan`'s rules, but those `refused`: a rule this ryo cannot run is left out, its
    /// refusal a finding, and the call is made again without it. ryo reads the project's files as given, `written` at
    /// `root`, and nothing of the disk.
    fn project_lint(
        &self,
        root: &MachinePath,
        written: &BTreeMap<RepoPath, FileText>,
        plan: &crate::domain::applications::types::Plan,
        corpus: &[RepoPath],
        findings: &mut Vec<crate::domain::findings::types::Finding>,
        refused: &mut BTreeSet<crate::domain::applications::types::LintRuleId>,
    ) -> Result<AnalysisReport, Problem> {
        // The corpus is relative to the plan's base; the files written are relative to the project's root.
        let given = corpus
            .iter()
            .filter_map(|path| {
                let at = join(&plan.base, PathText(&path.0)).unwrap_or_else(|| path.clone());
                written
                    .get(&at)
                    .map(|text| crate::outbound::analysis::types::GivenFile {
                        path: path.clone(),
                        text: crate::domain::contracts::types::RenderedContent(text.0.clone()),
                    })
            })
            .collect::<Vec<_>>();
        self.linted(root, plan, &given, corpus, findings, refused)
    }

    /// The lint call of `project_lint` over the files `given`, at the project's `root`, made again without each rule ryo
    /// cannot run.
    fn linted(
        &self,
        root: &MachinePath,
        plan: &crate::domain::applications::types::Plan,
        given: &[crate::outbound::analysis::types::GivenFile],
        corpus: &[RepoPath],
        findings: &mut Vec<crate::domain::findings::types::Finding>,
        refused: &mut BTreeSet<crate::domain::applications::types::LintRuleId>,
    ) -> Result<AnalysisReport, Problem> {
        use crate::domain::linting::service::{lint_root, runtime_refusal};
        use crate::domain::linting::types::RefusalReason;
        let sets = self.sets();
        loop {
            let rules = plan
                .rules
                .iter()
                .filter(|rule| !refused.contains(&rule.id))
                .cloned()
                .collect::<Vec<_>>();
            match self.analysis.lint(AnalysisRequest {
                root: lint_root(root, &plan.base),
                rules,
                placed: plan.placed.clone(),
                corpus: Corpus::Given(given.to_vec()),
                beside: plan
                    .contracts
                    .iter()
                    .map(|contract| crate::outbound::analysis::types::GivenFile {
                        path: contract.rendered.clone(),
                        text: contract.text.clone(),
                    })
                    .collect(),
                hosted: Hosted::Nothing,
            }) {
                Ok(report) => return Ok(report),
                Err(crate::outbound::analysis::errors::AnalysisError::Rule { rule, reason })
                    if !refused.contains(&rule) && plan.purposes.contains_key(&rule) =>
                {
                    let governance =
                        crate::domain::governance::service::governance(plan, &rule, corpus, &sets);
                    findings.extend(runtime_refusal(
                        plan,
                        &rule,
                        &RefusalReason(reason.into()),
                        governance,
                    ));
                    let _: bool = refused.insert(rule);
                }
                Err(other) => return Err(unchecked(other)),
            }
        }
    }

    /// The findings of the project's counts and pairs, as `check` makes them: a second lint call at the project's `root`,
    /// over the rows they host once the conditions they read have answered in `report`, its corpus empty; none when
    /// `plan` holds neither.
    fn project_counted(
        &self,
        root: &MachinePath,
        plan: &crate::domain::applications::types::Plan,
        report: &AnalysisReport,
        refused: &BTreeSet<crate::domain::applications::types::LintRuleId>,
    ) -> Result<Vec<crate::domain::findings::types::Finding>, Problem> {
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
        let counted = self
            .analysis
            .lint(AnalysisRequest {
                root: crate::domain::linting::service::lint_root(root, &plan.base),
                rules: crate::domain::judging::service::rules(),
                placed: Vec::new(),
                corpus: Corpus::Given(Vec::new()),
                beside: Vec::new(),
                hosted: Hosted::Counts(rows),
            })
            .map_err(unchecked)?;
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

    /// Reads the project's root `tastemaker.toml`, at the root of `repository`, the files it governs and its local files.
    fn project_inputs(&self, repository: &dyn Repository) -> Result<ProjectInputs, Problem> {
        use crate::domain::selectors::service::{covers_any, first_match, resolve};
        use crate::domain::selectors::types::{Coverage, SelectionContext};
        let directory = RepoPath::default();
        let file = join(&directory, PathText(WORKSPACE_FILE)).unwrap_or_default();
        let root_text = repository.read(&file).map_err(unchecked)?;
        let mut workspace = self
            .manifest
            .workspace_file(&DefinitionText(root_text.0.clone()))
            .map_err(unread(&file))?;
        let Role::Root(mut section) = workspace.role.clone() else {
            return Err(unchecked(format_args!(
                "`{}` is a local tastemaker.toml: run from the workspace whose root file names it in `members`",
                file.0
            )));
        };
        let mut problems = crate::domain::variables::service::resolve(
            &mut workspace.composition,
            &mut [&mut section.rules_exclude, &mut section.members],
        );
        problems.sort();
        workspace.role = Role::Root(section.clone());
        let compile = self.compile();
        let none = crate::domain::captures::types::Interpolations::default();
        // Every path of the root file is relative to its directory, the workspace's.
        let context = SelectionContext {
            base: &directory,
            values: &none,
            instances: &[],
            placed: &[],
            compile: &compile,
        };
        let selector = |source| {
            unchecked(crate::domain::placement::errors::PlacementError::Selector {
                entry: crate::domain::template_placing::constants::WORKSPACE_TABLE.to_owned(),
                source,
            })
        };
        let members = resolve(&section.members, &context).map_err(selector)?;
        let excluded = resolve(&section.rules_exclude, &context).map_err(selector)?;
        let member = |candidate: &RepoPath| {
            if first_match(&members, candidate).is_some() {
                Coverage::Covered
            } else {
                Coverage::Uncovered
            }
        };
        let files = crate::domain::definitions::service::governed(
            repository.walk(&directory).map_err(unchecked)?.files,
            &file,
            member,
            |path: &RepoPath| self.role_of(repository, path),
        )?;
        let mut locals = Vec::new();
        for path in crate::domain::definitions::service::locals(&files, &file, member, |path| {
            covers_any(&excluded, path)
        }) {
            locals.push(LocalText {
                path: path.clone(),
                text: repository.read(path).map_err(unchecked)?,
            });
        }
        Ok(ProjectInputs {
            repository: repository.root().clone(),
            directory,
            file,
            files,
            locals,
            section,
            workspace,
            variables: problems,
        })
    }

    /// The version a scenario's project uses for `release`: `subject`'s, the version proven, for its definition, and the
    /// latest its package offers for any other.
    fn scenario_version(
        &self,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        subject: Option<&Release>,
        release: &Release,
    ) -> Result<crate::domain::packages::types::VersionText, PackagesError> {
        if let Some(subject) = subject.filter(|subject| {
            subject.package == release.package
                && subject.kind == release.kind
                && subject.name == release.name
        }) {
            return Ok(subject.version.clone());
        }
        let none = crate::domain::packages::types::VersionText(Box::default());
        let Some(package) = packages.get(&release.package) else {
            return Ok(none);
        };
        let offered = self.offered(
            &package.directory,
            PackageFile(format!("{}/{}", kind_directory(release.kind), release.name.0).into()),
        )?;
        Ok(crate::domain::packages::service::latest(
            &none,
            offered.iter(),
        ))
    }

    /// The latest version each package offers of every definition `workspace` reads, for `#outdated`.
    ///
    /// # Errors
    ///
    /// The problem of a package whose versions cannot be listed.
    fn latest_offered(&self, workspace: &mut LoadedWorkspace) -> Result<(), Problem> {
        for release in crate::domain::definitions::service::shared(workspace) {
            let Some(package) = workspace.packages.get(&release.package) else {
                continue;
            };
            let offered = self
                .offered(
                    &package.directory,
                    PackageFile(
                        format!("{}/{}", kind_directory(release.kind), release.name.0).into(),
                    ),
                )
                .map_err(problem)?;
            let best = crate::domain::packages::service::latest(&release.version, offered.iter());
            drop(workspace.latest.insert(release, best));
        }
        Ok(())
    }

    /// Reads every definition the project names, and plans its lint call: an entry naming `subject`'s definition uses
    /// its version, and every other the latest its package offers.
    fn project_plan(
        &self,
        inputs: &ProjectInputs,
        subject: Option<&Release>,
    ) -> Result<ProjectPlan, Problem> {
        use crate::domain::definitions::service::{
            folded, local, package_paths, variable_findings,
        };
        let packages = self.project_packages(inputs)?;
        let mut findings = variable_findings(&inputs.variables, &inputs.file);
        findings.extend(folded(&inputs.workspace.composition, &inputs.file));
        let mut locals = Vec::new();
        for LocalText { path, text } in &inputs.locals {
            let mut file = self
                .manifest
                .workspace_file(&DefinitionText(text.0.clone()))
                .map_err(unread(path))?;
            if let Role::Root(_) = file.role {
                return Err(unchecked(format_args!(
                    "`{}`: a local tastemaker.toml never has a `workspace`: the root file declares it",
                    path.0
                )));
            }
            findings.extend(variable_findings(
                &crate::domain::variables::service::resolve(&mut file.composition, &mut []),
                path,
            ));
            findings.extend(folded(&file.composition, path));
            locals.push(local(path, file.composition));
        }
        // A version the package cannot list is a failure, said once the pins are made.
        let failed = std::cell::RefCell::new(None::<PackagesError>);
        let version_of =
            |release: &Release| match self.scenario_version(&packages, subject, release) {
                Ok(version) => version,
                Err(error) => {
                    if failed.borrow().is_none() {
                        *failed.borrow_mut() = Some(error);
                    }
                    crate::domain::packages::types::VersionText(Box::default())
                }
            };
        let mut composition = inputs.workspace.composition.clone();
        crate::domain::entries::service::pin(&mut composition, &version_of);
        for each in &mut locals {
            crate::domain::entries::service::pin(&mut each.composition, &version_of);
        }
        if let Some(error) = failed.borrow_mut().take() {
            return Err(problem(error));
        }
        let package_dirs = package_paths(
            &packages,
            crate::domain::definitions::types::RepositoryText(&inputs.repository.0),
        );
        let split = crate::domain::linting::service::partitioned(&inputs.files, &package_dirs);
        let mut loading = begin(LoadedWorkspace {
            file: inputs.file.clone(),
            directory: inputs.directory.clone(),
            section: inputs.section.clone(),
            composition,
            locals,
            packages,
            specs: BTreeMap::new(),
            rules: BTreeMap::new(),
            templates: BTreeMap::new(),
            latest: BTreeMap::new(),
            files: split.governed,
            local_packages: split.local_packages,
        });
        self.project_definitions(&mut loading, &mut findings)?;
        let crate::domain::loading::types::WorkspaceLoad {
            mut workspace,
            problems,
            ..
        } = finish(loading);
        if let Some(problem) = problems.first() {
            return Err(unchecked(&problem.0));
        }
        self.latest_offered(&mut workspace)?;
        let compile = self.compile();
        let placement = place(&workspace, &compile, &*self.manifest).map_err(unchecked)?;
        let mut plan = plan(
            &workspace,
            &placement,
            RepoPath::default(),
            crate::domain::applications::types::Compilers {
                compile: &compile,
                sets: &self.sets(),
            },
        )
        .map_err(unchecked)?;
        let contracts = self.project_contracts(&workspace, &placement, &mut findings);
        crate::domain::built_ins::service::compared(&mut plan, contracts);
        Ok(ProjectPlan { plan, findings })
    }

    /// The project's packages, direct and transitive, breadth first.
    fn project_packages(
        &self,
        inputs: &ProjectInputs,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, Problem> {
        let workspace_directory = PackageRoot(
            std::path::Path::new(&*inputs.repository.0)
                .join(&*inputs.directory.0)
                .display()
                .to_string()
                .into(),
        );
        let mut queue = inputs
            .section
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
            let located = self
                .packages
                .locate(&dependency, &from, &base)
                .map_err(unchecked)?;
            let manifest = self.package_manifest(&located.directory)?;
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

    /// The manifest of the package at `directory`, as a scenario's project reads it.
    fn package_manifest(
        &self,
        directory: &PackageRoot,
    ) -> Result<crate::domain::packages::types::PackageManifest, Problem> {
        let place = RepoPath(format!("{}/{PACKAGE_FILE}", directory.0).into());
        self.package_read(directory).map_err(|why| match why {
            super::types::Unread::File(reason) => unchecked(reason),
            super::types::Unread::Text(reason) => unread(&place)(reason),
        })
    }

    /// The manifest of the package at `directory`, as this run reads it: read and parsed by the first that asks, the
    /// workspace or a scenario's project, and kept for the others.
    fn package_read(
        &self,
        directory: &PackageRoot,
    ) -> Result<crate::domain::packages::types::PackageManifest, super::types::Unread> {
        let known = self
            .manifests
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(directory)
            .cloned();
        if let Some(known) = known {
            return (*known).clone();
        }
        let read = self
            .packages
            .read(directory, &PackageFile(PACKAGE_FILE.into()))
            .map_err(super::types::Unread::File)
            .and_then(|text| {
                self.manifest
                    .package(&DefinitionText(text.0))
                    .map_err(super::types::Unread::Text)
            });
        let mut kept = self
            .manifests
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if kept.len() < super::constants::MANIFESTS {
            drop(kept.insert(directory.clone(), Arc::new(read.clone())));
        }
        read
    }

    /// The versions the package at `directory` offers of the definition at `within`, as this run lists them: listed by
    /// the first that asks, and kept for the others.
    fn offered(
        &self,
        directory: &PackageRoot,
        within: PackageFile,
    ) -> Result<crate::outbound::packages::types::Versions, PackagesError> {
        let key = (directory.clone(), within);
        let known = self
            .offers
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned();
        if let Some(known) = known {
            return known;
        }
        let listed = self.packages.versions(directory, &key.1);
        let mut kept = self
            .offers
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if kept.len() < super::constants::OFFERS {
            drop(kept.insert(key, listed.clone()));
        }
        listed
    }

    /// Reads every definition the project's load asks for, until it asks for none; a spec's variable problems are
    /// findings. Each wave is read and parsed in parallel, then recorded in the order the load asked for it.
    fn project_definitions(
        &self,
        loading: &mut crate::domain::loading::types::Loading,
        findings: &mut Vec<crate::domain::findings::types::Finding>,
    ) -> Result<(), Problem> {
        loop {
            let wave = crate::domain::loading::service::wanted(loading);
            if wave.is_empty() {
                return Ok(());
            }
            let outcomes = wave
                .par_iter()
                .map(|wanted| self.project_definition(wanted))
                .collect::<Vec<_>>();
            for (wanted, outcome) in wave.iter().zip(outcomes) {
                let Parsed { definition, place } = outcome?;
                let problems = definition_read(loading, wanted, definition);
                findings.extend(crate::domain::definitions::service::variable_findings(
                    &problems, &place,
                ));
            }
        }
    }

    /// One definition of the project read and parsed, and the content file a template names: the one this run read
    /// before from the same file, else read now. A project reads no scenario of a definition (placement, planning and
    /// the report read its entries, its rules and its contents), so a definition is kept without them.
    fn project_definition(
        &self,
        wanted: &crate::domain::loading::types::Wanted,
    ) -> Result<Parsed, Problem> {
        let key = super::types::ParseKey {
            kind: wanted.release.kind,
            package: wanted.directory.clone(),
            file: wanted.file.clone(),
        };
        let known = self
            .parsed
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
            .cloned();
        if let Some(known) = known {
            return (*known).clone();
        }
        let place = RepoPath(format!("{}/{}", wanted.directory.0, wanted.file.0).into());
        let read = self
            .packages
            .read(&wanted.directory, &PackageFile(wanted.file.0.clone()))
            .map_err(unchecked)
            .and_then(|text| {
                self.manifest
                    .definition(wanted.release.kind, &DefinitionText(text.0))
                    .map_err(unread(&place))
            })
            .map(|mut definition| {
                match &mut definition {
                    Definition::Rule(rule) => rule.scenarios.clear(),
                    Definition::Spec(spec) => spec.scenarios.clear(),
                    Definition::Template(template) => template.scenarios.clear(),
                }
                Parsed { definition, place }
            });
        let mut kept = self
            .parsed
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if kept.len() < super::constants::PARSES {
            drop(kept.insert(key, Arc::new(read.clone())));
        }
        read
    }

    /// The project's placed files compared with their templates' contents, each rendered in parallel: the contracts,
    /// and the `#template` errors of the contents that could not be rendered.
    fn project_contracts(
        &self,
        workspace: &LoadedWorkspace,
        placement: &crate::domain::placement::types::Placement,
        failures: &mut Vec<crate::domain::findings::types::Finding>,
    ) -> Vec<crate::domain::contracts::types::Contract> {
        use crate::domain::contract_rendering::service::{rendered, requests};
        use crate::domain::contract_rendering::types::{Rendered, Requested};
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

    /// The packages of the workspace `located` finds: its root `tastemaker.toml` read, and every package it depends on,
    /// resolved.
    fn workspace_packages(
        &self,
        located: &dyn crate::outbound::repository::interface::FoundRepository,
    ) -> Result<BTreeMap<PackageName, ResolvedPackage>, ValidateError> {
        let repository = located;
        let file = join(located.workspace(), PathText(WORKSPACE_FILE)).unwrap_or_default();
        let text = repository.read(&file)?;
        let workspace = self
            .manifest
            .workspace_file(&DefinitionText(text.0))
            .map_err(manifest_error(ErrorFile(&file.0)))?;
        let Role::Root(section) = workspace.role else {
            return Err(ValidateError::NotARoot {
                file: file.0.to_string(),
            });
        };
        let project = std::path::Path::new(&*located.root().0).join(&*located.workspace().0);
        self.packages(&section, &PackageRoot(project.display().to_string().into()))
    }

    /// Each package's files, listed once: its manifest, its releases, its nested packages and its versions; any other file
    /// is unproven.
    fn listed(
        &self,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
    ) -> Result<
        BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
        ValidateError,
    > {
        let mut listed = BTreeMap::new();
        for (name, package) in packages {
            let held = self
                .packages
                .files(&package.directory, &PackageFile(Box::default()))?;
            let files = held
                .0
                .iter()
                .map(|each| crate::domain::packages::types::ReleaseFile(each.0.to_string()))
                .collect::<Vec<_>>();
            if let Some(stray) = crate::domain::packages::service::strays(&files).first() {
                return Err(ValidateError::Stray {
                    package: name.0.to_string(),
                    file: stray.0.clone(),
                });
            }
            drop(listed.insert(package.directory.clone(), files));
        }
        Ok(listed)
    }

    /// Every lock of the repository, read: what each workspace in it pins.
    fn locks(
        &self,
        repository: &dyn Repository,
    ) -> Result<Vec<crate::domain::packages::types::Lock>, ValidateError> {
        repository
            .list(&RepoPath(Box::default()))?
            .iter()
            .filter(|path| path.0.rsplit('/').next() == Some(LOCK_FILE))
            .map(|path| {
                let text = repository.read(path)?;
                self.manifest
                    .lock(&DefinitionText(text.0))
                    .map_err(manifest_error(ErrorFile(&path.0)))
            })
            .collect()
    }

    /// Refuses a version of a package the workspace holds by path that no lock of the repository reads and that is not
    /// its definition's latest: it is there for no one. A workspace with no lock of its own pins nothing yet, so what it
    /// would read is not known until `tm check --fix` writes one.
    fn unread(
        &self,
        located: &dyn crate::outbound::repository::interface::FoundRepository,
        packages: &BTreeMap<PackageName, ResolvedPackage>,
        listed: &BTreeMap<PackageRoot, Vec<crate::domain::packages::types::ReleaseFile>>,
    ) -> Result<(), ValidateError> {
        let repository = located;
        let own_lock = join(located.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
        if repository.exists(&own_lock) != Presence::Present {
            return Ok(());
        }
        let mut local = Vec::new();
        for (name, package) in packages {
            if !matches!(
                package.pin,
                crate::domain::packages::types::PackageSource::Path(_)
            ) {
                continue;
            }
            let files = listed
                .get(&package.directory)
                .map(Vec::as_slice)
                .unwrap_or_default();
            for (definition, offered) in crate::domain::packages::service::held_versions(files) {
                local.extend(offered.into_iter().map(|version| Release {
                    package: name.clone(),
                    kind: definition.kind,
                    name: definition.name.clone(),
                    version,
                    ownership: Ownership::Shared,
                }));
            }
        }
        // A workspace lock that does not read pins none, and `check` says why (and writes it again under `--fix`).
        let own = repository.read(&own_lock)?;
        if self.manifest.lock(&DefinitionText(own.0)).is_err() {
            return Ok(());
        }
        let locks = self.locks(repository)?;
        let unread = crate::domain::packages::service::unread(&local, &locks);
        if unread.is_empty() {
            return Ok(());
        }
        Err(ValidateError::Unread {
            versions: unread
                .iter()
                .map(|release| {
                    crate::domain::scenario_proving::service::subject_of(release)
                        .id
                        .0
                        .to_string()
                })
                .collect(),
        })
    }

    /// The versions the workspace's lock pins, in use beside every definition's latest; a lock that does not read pins
    /// none, and `check` says why.
    fn pinned(
        &self,
        located: &dyn crate::outbound::repository::interface::FoundRepository,
    ) -> BTreeSet<Release> {
        let repository = located;
        let lock = join(located.workspace(), PathText(LOCK_FILE)).unwrap_or_default();
        repository
            .read(&lock)
            .ok() // ignore: idiomatic.arch/no-ok-on-result - a lock that does not read pins none (the doc above), and `check` says why
            .and_then(|written| self.manifest.lock(&DefinitionText(written.0)).ok()) // ignore: idiomatic.arch/no-ok-on-result - a lock that does not parse pins none (the doc above), and `check` says why
            .map(|read| {
                read.packages
                    .into_iter()
                    .flat_map(|package| {
                        package.releases.into_iter().map(move |used| Release {
                            package: package.name.clone(),
                            kind: used.kind,
                            name: used.name,
                            version: used.version,
                            ownership: Ownership::Shared,
                        })
                    })
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default()
    }

    /// The proof of each version `asked` names, kept or made, in its order. Every version without a kept proof is proved at
    /// once inside the invocation's pool, so every scenario of every version shares its workers, and the work each
    /// scenario's check hands the pool stays in it.
    fn proved(
        &self,
        asked: &[AskedProof],
    ) -> Result<Vec<Result<Proof, ValidateError>>, ValidateError> {
        // Every proof held: nothing to prove, and no worker started.
        if asked.iter().all(|each| each.held.is_some()) {
            return Ok(asked
                .iter()
                .filter_map(|each| each.held.clone().map(Ok))
                .collect());
        }
        let mut made = None;
        let ran = self.scheduler.run(&mut || {
            made = Some(
                asked
                    .par_iter()
                    .map(|each| match &each.held {
                        Some(proof) => Ok(proof.clone()),
                        None => self.prove(&each.version),
                    })
                    .collect::<Vec<_>>(),
            );
        });
        // What this run read and judged is forgotten: the next run reads the definitions as they are then.
        drop(std::mem::take(
            &mut *self
                .parsed
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ));
        drop(std::mem::take(
            &mut *self
                .manifests
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ));
        // What this run listed and was told by ryo is forgotten too: the packages may offer more by the next, and a verdict is
        // asked again.
        drop(std::mem::take(
            &mut *self
                .offers
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ));
        drop(std::mem::take(
            &mut *self
                .verdicts
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ));
        // Every wrapper package this run made is forgotten.
        drop(std::mem::take(
            &mut *self
                .wrappers
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ));
        ran?;
        made.ok_or_else(|| {
            ValidateError::Scheduler(
                crate::outbound::scheduler::errors::SchedulerError::Unavailable {
                    reason: crate::domain::proofs::constants::UNEXECUTED_PROOFS.into(),
                },
            )
        })
    }
}

impl ValidateUseCase for ValidateUseCaseImpl {
    fn validate(&self, request: ValidateRequest) -> Result<ValidateReport, ValidateError> {
        let located = self.repository.find(&request.start)?;
        let packages = self.workspace_packages(located.as_ref())?;
        let kept = self
            .store
            .definitions()
            .0
            .into_iter()
            .map(|row| (row.release.clone(), row))
            .collect::<BTreeMap<_, _>>();
        let listed = self.listed(&packages)?;
        self.unread(located.as_ref(), &packages, &listed)?;
        let mut proved = Vec::new();
        let versions = versions(&packages, &listed, &self.pinned(located.as_ref()));
        let asked_for = |version: &Version| {
            request.only.is_empty()
                || request
                    .only
                    .iter()
                    .any(|only| version.subject.id.0.contains(&*only.0))
        };
        let digesting = self.digests(&versions, &request.only, &packages, &listed, &kept)?;
        let mut rows = BTreeMap::new();
        let mut asked = Vec::new();
        for version in versions {
            let key = digesting
                .done
                .get(&version.release)
                .cloned()
                .unwrap_or_default();
            // A row is the proof: a version whose digest moved holds no more until it is proved under the new one.
            let held = kept.get(&version.release).filter(|row| row.digest == key);
            let stamp = digesting
                .kept
                .get(&version.release)
                .copied()
                .unwrap_or_default();
            if let Some(row) = held {
                drop(rows.insert(
                    version.release.clone(),
                    ProvenVersion {
                        stamp,
                        ..row.clone()
                    },
                ));
            }
            if !asked_for(&version) {
                continue;
            }
            let held = held.map(|_| Proof::Proven);
            asked.push(AskedProof { version, key, held });
        }
        // The versions whose proof is not held are proved at once, all their scenarios sharing the one pool, and answered
        // in their order.
        let answers = self.proved(&asked)?;
        let mut failed = false;
        for (each, answer) in asked.into_iter().zip(answers) {
            let proof = answer?;
            let proving = if each.held.is_some() {
                Proving::Kept
            } else {
                Proving::Made
            };
            // A version that held is kept; one that did not is not, so it is proved again next run.
            if proof == Proof::Proven {
                drop(
                    rows.insert(
                        each.version.release.clone(),
                        ProvenVersion {
                            release: each.version.release.clone(),
                            stamp: digesting
                                .kept
                                .get(&each.version.release)
                                .copied()
                                .unwrap_or_default(),
                            digest: each.key,
                            uses: digesting
                                .uses
                                .get(&each.version.release)
                                .cloned()
                                .unwrap_or_default(),
                        },
                    ),
                );
            } else {
                drop(rows.remove(&each.version.release));
                failed = true;
            }
            proved.push(Proved {
                subject: each.version.subject,
                proof,
                proving,
            });
        }
        if !request.only.is_empty() && proved.is_empty() {
            return Err(ValidateError::NoMatch);
        }
        // A run that fails writes nothing: the store keeps what the last run that succeeded proved. Rows this run did not
        // change are not written again: an unchanged validation reads the store and writes nothing. A store that cannot be
        // written costs the next run its proofs, never this run its answer: the failure is said on stderr; under
        // `-n/--dry-run` nothing is written.
        let rows = rows.into_values().collect::<Vec<_>>();
        if !failed
            && !rows.iter().eq(kept.values())
            && let Err(error) = self.store.keep_definitions(&rows)
        {
            tracing::warn!(message = %error);
        }
        Ok(ValidateReport { proved })
    }
}
