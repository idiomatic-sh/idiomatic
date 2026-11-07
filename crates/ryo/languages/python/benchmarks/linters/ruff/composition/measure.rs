//! The measure commands: certify both contestants, then compare them on the pinned corpus.
use super::certification::{
    Certifying, MeasureCommand, MeasurePaths, Selection, StagedSuite, RuleTally, announce,
    candidate_linter, cases_for, held_to, paths, scope_of, stage,
};
use super::commands;
use super::constants;
use super::constants::UNEXPECTED_MEASUREMENT_ARGUMENT;
use super::corpus::{Corpus, SourceProfile};
use super::output;
use super::request_input::BenchmarkArgument;
use ryo_benchmark::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::certify::{
    interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::compare::use_case::CompareUseCaseImpl;
use ryo_benchmark::inbound::use_cases::measure::use_case::MeasureUseCaseImpl;
use ryo_benchmark::inbound::use_cases::publish::{
    interface::PublishUseCase as _, use_case::PublishUseCaseImpl,
};
use ryo_benchmark::outbound::evidence::interface::EvidencePort;
use ryo_benchmark::outbound::execution::interface::ExecutionPort;
use ryo_benchmark::outbound::linter::interface::LinterPort;
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::context::types::RuleContextDeclaration;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::workloads::types::{Project, Suite};
use std::{path::PathBuf, sync::Arc};

/// Certify both contestants, then measure every project of the corpus and publish or report the samples.
pub(super) fn measure(
    command: MeasureCommand<'_>,
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        return Err(Error::Invalid(constants::RELEASE_REQUIRED.into()));
    }
    let MeasurePaths {
        executable,
        cache,
        collector_path,
    } = paths(args, suite_source)?;
    let mut tool = suite_source.profile(ArtifactPath(&commands::directory()))?;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let declared_inventory = tool.defaults.clone();
    let audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(&suite_source),
    )
    .audit(&commands::directory(), &tool)?;
    let authored_total = audit.authored_count();
    let selection_or_inventory = args.next().map(|argument| argument.0);
    let scope = scope_of(command);
    let Certifying {
        audit,
        cases: case_source,
    } = cases_for(
        scope,
        audit,
        selection_or_inventory.as_deref().map(Selection),
        &cache,
        suite_source,
    )?;
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_MEASUREMENT_ARGUMENT.into()));
    }
    let measurement_mode = held_to(scope, &audit, &tool, declared_inventory)?;
    let tally = RuleTally::of(&tool, authored_total);
    let project_context_mode = tool.project_context_mode;
    let project_search_roots = tool.project_search_roots.clone();
    tool.defaults = audit.authored().to_vec();
    let declared_rule_context = tool.rule_context.clone();
    let configured_cohort = Some(tool.defaults.clone());
    let StagedSuite {
        scratch: selected_scratch,
        profile: selected_profile,
        workload: frozen_workload,
    } = stage(suite_source, &cache, audit.authored())?;
    let selected_rules = selected_profile.join(crate::constants::RULES_DIRECTORY);
    let candidate_executable = selected_scratch.join(constants::CANDIDATE_EXECUTABLE);
    suite_source.copy_file(
        ArtifactPath(&std::env::current_exe()?),
        ArtifactPath(&candidate_executable),
    )?;
    let candidate = candidate_linter(&tool, candidate_executable.clone(), selected_rules.clone());
    let reference = Arc::new(commands::reference(executable.clone(), tool)?);
    let collector_settings =
        commands::collector(suite_source.as_ref(), &collector_path, &frozen_workload)?;
    let execution = commands::timed(collector_settings, observations)?;
    let case_scratch = suite_source.scratch(
        ArtifactPath(std::path::Path::new(constants::DEFAULT_SCRATCH_ROOT)),
        constants::CASE_SCRATCH,
    )?;
    let certificate = CertifyUseCaseImpl::new(
        Arc::<_>::clone(&suite_source),
        Arc::<super::super::adapter::LinterRuffAdapter>::clone(&reference),
        Arc::<_>::clone(&candidate),
        Arc::<_>::clone(&execution),
        Arc::new(ryo_benchmark::outbound::inventory::factory::local()),
    )
    .certify(
        &selected_profile,
        &frozen_workload,
        &executable,
        &candidate_executable,
        &selected_rules,
        &case_scratch,
        scope,
        &case_source,
    )?;
    suite_source.discard(ArtifactPath(&case_scratch))?;
    announce(
        observations,
        scope,
        &tally,
        &audit,
        &certificate,
        &selected_profile,
    )?;
    output::environment(observations)?;
    compare(
        Certified {
            reference,
            candidate,
            execution,
            certificate,
            cohort: configured_cohort,
            rule_context: declared_rule_context,
            search_roots: project_search_roots,
            context_mode: project_context_mode,
            mode: measurement_mode,
            cache,
            workload: frozen_workload,
            scope,
        },
        command,
        suite_source,
        observations,
    )
}

/// Measure every project of the corpus, then publish the samples of an accounted run or say that a diagnostic one is not a metric.
fn finish(
    corpus: &Corpus<'_>,
    projects: Vec<Project>,
    scope: BenchmarkCoverage,
    evidence: &Arc<dyn EvidencePort>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let mut samples = Vec::new();
    let mut failures = Vec::new();
    for project in projects {
        match corpus.measure_project(&project, observations) {
            Ok(observed) => samples.extend(observed),
            Err(error) => {
                writeln!(
                    observations,
                    "{}: corpus comparison failed: {error}",
                    project.name
                )?;
                failures.push(project.name);
            }
        }
    }
    // The evidence boundary refuses an empty observation set; never bypass that guard.
    if scope == BenchmarkCoverage::Accounted {
        PublishUseCaseImpl::new(Arc::clone(evidence)).publish(&samples)?;
    } else {
        writeln!(
            observations,
            "diagnostic-only: scoped samples are not benchmark metrics"
        )?;
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "corpus comparisons failed: {failures:?}"
        )))
    }
}

/// What certification leaves for the comparison: both contestants, the certificate they hold and where the corpus is measured.
struct Certified {
    reference: Arc<super::super::adapter::LinterRuffAdapter>,
    candidate: Arc<dyn LinterPort>,
    execution: Arc<dyn ExecutionPort>,
    certificate: Certificate,
    cohort: Option<Vec<ryo::linter::RuleId>>,
    rule_context: Option<RuleContextDeclaration>,
    search_roots: Option<Vec<ryo::linter::RelativePath>>,
    context_mode: ProjectContextMode,
    mode: MeasurementMode,
    cache: PathBuf,
    workload: PathBuf,
    scope: BenchmarkCoverage,
}

/// Measure the corpus of a certified pair and publish or report the samples.
fn compare(
    certified: Certified,
    command: MeasureCommand<'_>,
    suite_source: &Arc<dyn SuitePort>,
    observations: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let Certified {
        reference,
        candidate,
        execution,
        certificate,
        cohort,
        rule_context,
        search_roots,
        context_mode,
        mode,
        cache,
        workload,
        scope,
    } = certified;
    let suite: Suite = suite_source.projects(ArtifactPath(&workload))?;
    if suite.language.0 != constants::PYTHON || !(2..=3).contains(&suite.projects.len()) {
        return Err(Error::Invalid(constants::PROJECT_COUNT_ERROR.into()));
    }
    let measurements = MeasureUseCaseImpl::new(
        Arc::<super::super::adapter::LinterRuffAdapter>::clone(&reference),
        Arc::<_>::clone(&candidate),
        Arc::<_>::clone(&execution),
        Arc::<_>::clone(&suite_source),
        mode,
    );
    let evidence: Arc<dyn EvidencePort> = Arc::new(
        ryo_benchmark::outbound::evidence::factory::bmf(Box::new(std::io::stdout())),
    );
    let comparison = CompareUseCaseImpl::new();
    let profile = if matches!(
        command.0,
        constants::MEASURE_VALID | constants::MEASURE_ACCOUNTED_VALID
    ) {
        SourceProfile::Valid
    } else {
        SourceProfile::Full
    };
    let corpus = Corpus {
        profile,
        cache: &cache,
        frozen_workload: &workload,
        suite_source: suite_source.as_ref(),
        reference: reference.as_ref(),
        candidate: candidate.as_ref(),
        execution: execution.as_ref(),
        certificate: &certificate,
        measurements: &measurements,
        comparison: &comparison,
        configured_cohort: &cohort,
        declared_rule_context: &rule_context,
        project_search_roots: &search_roots,
        project_context_mode: context_mode,
    };
    finish(&corpus, suite.projects, scope, &evidence, observations)
}
