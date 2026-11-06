//! The measure commands: certify both contestants, then compare them on the pinned corpus.
use super::commands;
use super::constants;
use super::constants::{
    CANONICAL_BENCHMARK_REQUIRES_EXPLI, CASE_INVENTORY, REGISTERED_CASE_RESULTS,
    UNEXPECTED_MEASUREMENT_ARGUMENT,
};
use super::corpus::{Corpus, SourceProfile};
use super::output;
use super::request_input::BenchmarkArgument;
use ryo_benchmark::domain::suite::service::require_accounted;
use ryo_benchmark::domain::suite::types::audit::value::Audit;
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
use ryo_benchmark::inbound::use_cases::stage_suite::{
    interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl,
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
use ryo_benchmark_contracts::domain::requests::types::{BenchmarkCoverage, CertificationCases};
use ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount;
use ryo_benchmark_contracts::domain::workloads::types::{Project, StagePurpose, Suite, Tool};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The first argument of a measure invocation.
#[derive(Clone, Copy, Debug)]
pub(super) struct MeasureCommand<'a>(pub(super) &'a str);

/// Certify both contestants, then measure every project of the corpus and publish or report the samples.
pub(super) fn measure(
    command: MeasureCommand<'_>,
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    let command = command.0;
    if cfg!(debug_assertions) {
        return Err(Error::Invalid(constants::RELEASE_REQUIRED.into()));
    }
    let MeasurePaths {
        executable,
        cache,
        collector_path,
    } = paths(args)?;
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
    let (audit, case_source) = cases_for(scope, audit, selection_or_inventory.as_deref(), &cache)?;
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_MEASUREMENT_ARGUMENT.into()));
    }
    let measurement_mode = held_to(scope, &audit, &tool, declared_inventory)?;
    let tally = Tally::of(&tool, authored_total);
    let project_context_mode = tool.project_context_mode;
    let project_search_roots = tool.project_search_roots.clone();
    tool.defaults = audit.authored().to_vec();
    let declared_rule_context = tool.rule_context.clone();
    let configured_cohort = Some(tool.defaults.clone());
    let Staged {
        scratch: selected_scratch,
        profile: selected_profile,
        workload: frozen_workload,
    } = stage(suite_source, &cache, audit.authored())?;
    let selected_rules = selected_profile.join(constants::RULES_DIRECTORY);
    let candidate_executable = selected_scratch.join(constants::CANDIDATE_EXECUTABLE);
    let _bytes = std::fs::copy(std::env::current_exe()?, &candidate_executable)?;
    let candidate = candidate_linter(&tool, candidate_executable.clone(), selected_rules.clone());
    let reference = Arc::new(commands::reference(executable.clone(), tool)?);
    let collector_settings =
        commands::collector(suite_source.as_ref(), &collector_path, &frozen_workload)?;
    let execution = commands::timed(collector_settings, observations)?;
    let case_scratch = tempfile::tempdir()?;
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
        case_scratch.path(),
        scope,
        &case_source,
    )?;
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

/// The three executables and the cache directory a measure invocation names, in the order it names them.
struct MeasurePaths {
    executable: PathBuf,
    cache: PathBuf,
    collector_path: PathBuf,
}

/// Read the reference executable, the cache directory and the collector from the arguments.
fn paths(args: &mut dyn Iterator<Item = BenchmarkArgument>) -> Result<MeasurePaths, Error> {
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let cache = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_CACHE.into()))?,
    );
    let collector_path = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_COLLECTOR.into()))?,
    )
    .canonicalize()?;
    std::fs::create_dir_all(&cache)?;
    let cache = cache.canonicalize()?;
    Ok(MeasurePaths {
        executable,
        cache,
        collector_path,
    })
}

/// The registered case inventory an accounted measurement is held to, with the scratch directory for its results.
fn registered_cases(
    selection_or_inventory: Option<&str>,
    cache: &Path,
) -> Result<CertificationCases, Error> {
    let input = selection_or_inventory
        .and_then(|s| s.strip_prefix(CASE_INVENTORY))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Invalid(CANONICAL_BENCHMARK_REQUIRES_EXPLI.into()))?;
    Ok(CertificationCases::Registered {
        registration: PathBuf::from(input).canonicalize()?,
        output: tempfile::Builder::new()
            .prefix(REGISTERED_CASE_RESULTS)
            .tempdir_in(cache)?
            .keep(),
    })
}

/// The directories one certification stages: its scratch root, the suite profile and the frozen workload.
struct Staged {
    scratch: PathBuf,
    profile: PathBuf,
    workload: PathBuf,
}

/// Stage the suite of the authored rules and its projects into a fresh scratch directory under the cache.
fn stage(
    suite_source: &Arc<dyn SuitePort>,
    cache: &Path,
    authored: &[ryo::linter::RuleId],
) -> Result<Staged, Error> {
    let scratch = tempfile::Builder::new()
        .prefix(constants::CERTIFICATION_PREFIX)
        .tempdir_in(cache)?
        .keep();
    let profile = scratch.join(constants::PROFILE_DIRECTORY);
    StageSuiteUseCaseImpl::new(suite_source.as_ref()).stage_suite(
        &commands::directory(),
        &profile,
        authored,
        StagePurpose::Execution,
    )?;
    let workload = scratch.join(constants::WORKLOAD_DIRECTORY);
    suite_source.stage_projects(
        ArtifactPath(&commands::directory().join(constants::LANGUAGE_DIRECTORY)),
        ArtifactPath(&workload),
    )?;
    Ok(Staged {
        scratch,
        profile,
        workload,
    })
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

/// The Ryo contestant: this executable bound to the staged rules and to the settings the reference tool declares.
fn candidate_linter(
    tool: &Tool,
    executable: PathBuf,
    rules: PathBuf,
) -> Arc<dyn ryo_benchmark::outbound::linter::interface::LinterPort> {
    Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
        executable,
        rules,
        tool.query_modules.clone(),
        tool.rule_applicability.clone(),
        tool.parser_configuration.clone(),
        tool.source_language.clone(),
        tool.project_context_mode,
        tool.defaults.clone(),
        tool.rule_observation_selection,
        tool.rule_observations.clone(),
    ))
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
    command: &str,
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
        command,
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

/// Whether a measure command holds the run to the accounted inventory or only to the rules it selects.
fn scope_of(command: &str) -> BenchmarkCoverage {
    if matches!(
        command,
        constants::MEASURE | constants::MEASURE_ACCOUNTED_VALID
    ) {
        BenchmarkCoverage::Accounted
    } else {
        BenchmarkCoverage::Interim
    }
}

/// Narrow the audit to the rules the invocation selects, and certify against the profile's own cases.
fn profile_cases(
    audit: Audit,
    selection: Option<&str>,
) -> Result<(Audit, CertificationCases), Error> {
    let audit = match selection {
        Some(selected) => audit.select(
            &selected
                .split(',')
                .map(ryo::linter::RuleId::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::Invalid(e.to_string()))?,
        )?,
        None => audit,
    };
    Ok((audit, CertificationCases::Profile))
}

/// An accounted run is held to the declared inventory; a diagnostic one to nothing.
fn measurement_mode(
    scope: BenchmarkCoverage,
    inventory: Vec<ryo::linter::RuleId>,
) -> MeasurementMode {
    if scope == BenchmarkCoverage::Accounted {
        MeasurementMode::Benchmark { inventory }
    } else {
        MeasurementMode::Diagnostic
    }
}

/// How many rules the tool ships, defers and leaves pending, and how many of the shipped ones are authored.
struct Tally {
    defaults: usize,
    deferred: usize,
    pending: usize,
    authored: RuleCount,
}

/// Say what the certification verified, and what it left unassessed.
fn announce(
    observations: &mut dyn std::io::Write,
    scope: BenchmarkCoverage,
    tally: &Tally,
    audit: &Audit,
    certificate: &Certificate,
    profile: &Path,
) -> Result<(), Error> {
    let Tally {
        defaults,
        deferred,
        pending,
        authored,
    } = tally;
    writeln!(
        observations,
        "scope: {}; {} verified / {defaults} shipped defaults; {deferred} assessed deferrals; {pending} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
        match scope {
            BenchmarkCoverage::Accounted => constants::ACCOUNTED,
            BenchmarkCoverage::Interim => constants::INTERIM,
        },
        audit.authored_count(),
        defaults - authored.0 - deferred - pending,
        authored.0 - audit.authored_count().0,
        certificate.cases(),
        certificate.fingerprint(),
        profile.display()
    )?;
    Ok(())
}

/// The mode the run is measured in, after the accounted run has shown its inventory and every run its authored rules.
fn held_to(
    scope: BenchmarkCoverage,
    audit: &Audit,
    tool: &Tool,
    inventory: Vec<ryo::linter::RuleId>,
) -> Result<MeasurementMode, Error> {
    if scope == BenchmarkCoverage::Accounted {
        require_accounted(audit, tool)?;
    }
    let mode = measurement_mode(scope, inventory);
    ryo_benchmark::domain::measurements::service::require_inventory(
        &mode,
        audit.authored(),
        Some(audit.authored()),
    )?;
    Ok(mode)
}

impl Tally {
    /// Count the tool's defaults, deferrals and pending rules beside the authored total.
    fn of(tool: &Tool, authored: RuleCount) -> Self {
        Self {
            defaults: tool.defaults.len(),
            deferred: tool.deferred.len(),
            pending: tool.pending.len(),
            authored,
        }
    }
}

/// The cases the run is certified against, and the audit narrowed to what it selects.
fn cases_for(
    scope: BenchmarkCoverage,
    audit: Audit,
    selection_or_inventory: Option<&str>,
    cache: &Path,
) -> Result<(Audit, CertificationCases), Error> {
    if scope == BenchmarkCoverage::Accounted {
        Ok((audit, registered_cases(selection_or_inventory, cache)?))
    } else {
        profile_cases(audit, selection_or_inventory)
    }
}
