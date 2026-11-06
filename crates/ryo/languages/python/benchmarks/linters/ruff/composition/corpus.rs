//! The pinned corpus: every project is prepared, measured in both orders and compared cold, retained and edited.
use super::commands;
use super::constants;
use super::output;
use ryo::linter::{RelativePath, RuleId};
use ryo_benchmark::application::run::service::run as execute;
use ryo_benchmark::domain::measurements::types::sample::value::BenchmarkSample;
use ryo_benchmark::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark::inbound::use_cases::compare::{
    interface::CompareUseCase as _, use_case::CompareUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::measure::{
    interface::MeasureUseCase as _, use_case::MeasureUseCaseImpl,
};
use ryo_benchmark::outbound::execution::interface::ExecutionPort;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::ArtifactSet;
use ryo_benchmark::outbound::workspace::interface::WorkspacePort as _;
use ryo_benchmark_contracts::domain::context::types::RuleContextDeclaration;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::RunOrder;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;
use ryo_benchmark_contracts::domain::workloads::types::Project;
use std::path::Path;

/// Which sources of a project are measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceProfile {
    /// Only sources the pinned Ruff parser accepts.
    Valid,
    /// Every discovered Python source.
    Full,
}

/// What every project of the corpus is measured with.
pub(super) struct Corpus<'a> {
    /// Which sources are measured.
    pub(super) profile: SourceProfile,
    /// Where projects and trials live.
    pub(super) cache: &'a Path,
    /// The frozen workload holding each project's patch.
    pub(super) frozen_workload: &'a Path,
    /// Artifact validation and fingerprinting.
    pub(super) suite_source: &'a dyn SuitePort,
    /// The pinned reference linter.
    pub(super) reference: &'a super::super::adapter::LinterRuffAdapter,
    /// The candidate linter.
    pub(super) candidate: &'a dyn LinterPort,
    /// Process launch and observation.
    pub(super) execution: &'a dyn ExecutionPort,
    /// The certificate both contestants hold.
    pub(super) certificate: &'a Certificate,
    /// The paired measurement operation.
    pub(super) measurements: &'a MeasureUseCaseImpl,
    /// The report comparison operation.
    pub(super) comparison: &'a CompareUseCaseImpl,
    /// The rules both contestants run.
    pub(super) configured_cohort: &'a Option<Vec<RuleId>>,
    /// The declared rule context.
    pub(super) declared_rule_context: &'a Option<RuleContextDeclaration>,
    /// The declared project search roots.
    pub(super) project_search_roots: &'a Option<Vec<RelativePath>>,
    /// How project context is sealed.
    pub(super) project_context_mode: ProjectContextMode,
}

impl Corpus<'_> {
    /// Measure one project in both orders, cold, retained and edited, and write what was observed.
    pub(super) fn measure_project(
        &self,
        project: &Project,
        observations: &mut dyn std::io::Write,
    ) -> Result<Vec<BenchmarkSample>, Error> {
        let mut project_samples = Vec::new();
        let workspaces = ryo_benchmark::outbound::workspace::factory::git(
            self.cache.join(constants::PROJECTS_DIRECTORY),
        );
        let mut workspace = workspaces.prepare(&project)?;
        let discovered: Vec<ryo::linter::RelativePath> = self
            .reference
            .files(workspace.root())?
            .into_iter()
            .collect();
        let (mut files, excluded): (Vec<_>, Vec<_>) = discovered.into_iter().partition(|file| {
            file.0.ends_with(constants::PYTHON_EXTENSION)
                || file.0.ends_with(constants::STUB_EXTENSION)
        });
        writeln!(
            observations,
            "{}: matched Python source scope {} files; excluded default-discovered formats {} files (source-kind capability gap): {}",
            project.name,
            files.len(),
            excluded.len(),
            excluded
                .iter()
                .map(|path| path.0.as_ref())
                .collect::<Vec<_>>()
                .join(constants::PATH_SEPARATOR)
        )?;
        let tracked = workspaces.tracked_files(&workspace)?.0;
        let tracked_paths: Vec<_> = tracked.iter().map(std::path::PathBuf::from).collect();
        if files.iter().any(|file| {
            !tracked
                .iter()
                .any(|tracked| tracked.as_str() == file.0.as_ref())
        }) {
            return Err(Error::Invalid(constants::UNTRACKED_SOURCE.into()));
        }
        if self.profile == SourceProfile::Valid {
            let preflight = RunRequest {
                performance: Default::default(),
                selected_rules: self.configured_cohort.clone(),
                rule_observations: None,
                rule_context: ryo_benchmark::domain::suite::rule_context::resolve(
                    self.declared_rule_context.as_ref(),
                    None,
                )?,
                project_search_roots: self.project_search_roots.clone(),
                given_project_inventory: Default::default(),
                staged_sources: None,
                file_metadata: None,
                context_manifest: commands::seal_context(
                    self.suite_source,
                    self.project_context_mode,
                    workspace.root(),
                    &files,
                    &tracked_paths,
                )?,
                root: workspace.root().to_owned(),
                files: files.clone(),
                state: self
                    .cache
                    .join(constants::PREFLIGHT_DIRECTORY)
                    .join(&project.name.0),
            };
            let parsed = execute(self.reference, self.execution, &preflight)?;
            let invalid: std::collections::BTreeSet<_> = parsed
                .report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule.as_ref() == constants::INVALID_SYNTAX)
                .map(|finding| finding.file.clone())
                .collect();
            for finding in parsed
                .report
                .diagnostics
                .iter()
                .filter(|finding| finding.rule.as_ref() == constants::INVALID_SYNTAX)
            {
                writeln!(
                    observations,
                    "{}: excluded invalid Python (pinned Ruff parser), path={} range={}..{}; frontend parser capability gap",
                    project.name, finding.file.0, finding.start.0, finding.end.0
                )?;
            }
            files.retain(|file| !invalid.contains(file));
        }
        let paths: Vec<_> = files
            .iter()
            .map(|file| workspace.root().join(file.0.as_ref()))
            .collect();
        let bytes = paths
            .iter()
            .map(|path| std::fs::metadata(path).map(|metadata| metadata.len()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum::<u64>();
        writeln!(
            observations,
            "{}: measured source files={} bytes={bytes} digest={} input-profile={}",
            project.name,
            files.len(),
            self.suite_source.fingerprint(ArtifactSet(&paths))?,
            if self.profile == SourceProfile::Valid {
                constants::VALID_SOURCE_PROFILE
            } else {
                constants::FULL_SOURCE_PROFILE
            }
        )?;
        for order in [RunOrder::ReferenceFirst, RunOrder::CandidateFirst]
            .into_iter()
            .cycle()
            .take(6)
        {
            // A new owned directory makes both retained and clean baselines fresh,
            // even if an earlier trial left caches behind in the caller's root.
            let trial = tempfile::Builder::new()
                .prefix(constants::TRIAL_PREFIX)
                .tempdir_in(self.cache)?
                .keep();
            let request = RunRequest {
                performance: Default::default(),
                selected_rules: self.configured_cohort.clone(),
                rule_observations: None,
                rule_context: ryo_benchmark::domain::suite::rule_context::resolve(
                    self.declared_rule_context.as_ref(),
                    None,
                )?,
                project_search_roots: self.project_search_roots.clone(),
                given_project_inventory: Default::default(),
                staged_sources: None,
                file_metadata: None,
                context_manifest: commands::seal_context(
                    self.suite_source,
                    self.project_context_mode,
                    workspace.root(),
                    &files,
                    &tracked_paths,
                )?,
                root: workspace.root().to_owned(),
                files: files.clone(),
                state: trial.join(constants::RETAINED_DIRECTORY),
            };
            let cold = self
                .measurements
                .measure(self.certificate, &request, order)?;
            let counts = output::diagnostic_counts(&cold.reference().report);
            writeln!(
                observations,
                "{}: exact matching findings per rule: {counts}",
                project.name
            )?;
            let retained = self
                .measurements
                .measure(self.certificate, &request, order)?;
            self.comparison
                .compare(&cold.reference().report, &retained.reference().report)
                .map_err(|difference| {
                    Error::Invalid(format!("reference retained-cache mismatch: {difference:?}"))
                })?;
            self.comparison
                .compare(&cold.candidate().report, &retained.candidate().report)
                .map_err(|difference| {
                    Error::Invalid(format!("candidate retained-cache mismatch: {difference:?}"))
                })?;
            workspaces.apply_patch(&mut workspace, &self.frozen_workload.join(&project.patch))?;
            let edited_request = RunRequest {
                context_manifest: commands::seal_context(
                    self.suite_source,
                    self.project_context_mode,
                    workspace.root(),
                    &files,
                    &tracked_paths,
                )?,
                ..request.clone()
            };
            let clean_request = RunRequest {
                state: trial.join(constants::CLEAN_EDITED_DIRECTORY),
                ..edited_request.clone()
            };
            let edited = self
                .measurements
                .measure(self.certificate, &edited_request, order);
            let clean_reference = execute(self.reference, self.execution, &clean_request);
            let clean_candidate = execute(self.candidate, self.execution, &clean_request);
            workspaces.reset(&mut workspace)?;
            let edited = edited?;
            self.comparison
                .compare(&edited.reference().report, &clean_reference?.report)
                .map_err(|difference| {
                    Error::Invalid(format!(
                        "reference edited retained vs fresh mismatch: {difference:?}"
                    ))
                })?;
            self.comparison
                .compare(&edited.candidate().report, &clean_candidate?.report)
                .map_err(|difference| {
                    Error::Invalid(format!(
                        "candidate edited retained vs fresh mismatch: {difference:?}"
                    ))
                })?;
            for (condition, pair) in [
                (constants::EMPTY_CACHE_CONDITION, cold),
                (constants::RETAINED_CACHE_CONDITION, retained),
                (constants::EDITED_CACHE_CONDITION, edited),
            ] {
                let contestants = [constants::REFERENCE_NAME, constants::CANDIDATE_NAME];
                let named = pair.samples(contestants.map(|contestant| {
                    ryo_benchmark_contracts::domain::measurements::types::SampleName(format!(
                        "{}/{contestant}/{condition}",
                        project.name
                    ))
                }));
                for (contestant, sample) in contestants.into_iter().zip(named) {
                    let observed_counts = output::diagnostic_counts(&sample.measurement().report);
                    writeln!(
                        observations,
                        "findings: project={} contestant={contestant} condition={condition} counts={observed_counts}",
                        project.name
                    )?;
                    output::observe_sample(observations, &sample, order)?;
                    project_samples.push(sample);
                }
            }
        }
        Ok(project_samples)
    }
}
