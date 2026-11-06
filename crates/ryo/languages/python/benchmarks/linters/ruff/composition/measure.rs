//! The measure commands: certify both contestants, then compare them on the pinned corpus.
use super::commands;
use super::constants;
use super::constants::{
    CANONICAL_BENCHMARK_REQUIRES_EXPLI, CASE_INVENTORY, REGISTERED_CASE_RESULTS,
    UNEXPECTED_MEASUREMENT_ARGUMENT,
};
use super::corpus::{Corpus, SourceProfile};
use super::output;
use ryo_benchmark::domain::suite::service::require_accounted;
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::certify::{
    interface::CertifyUseCase as _, use_case::CertifyUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::compare::use_case::CompareUseCaseImpl;
use ryo_benchmark::inbound::use_cases::measure::use_case::MeasureUseCaseImpl;
use ryo_benchmark::inbound::use_cases::stage_suite::interface::StageSuiteUseCase as _;
use ryo_benchmark::outbound::evidence::interface::EvidencePort;
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::ArtifactPath;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;
use ryo_benchmark_contracts::domain::workloads::types::Suite;
use std::{path::PathBuf, sync::Arc};

/// Certify both contestants, then measure every project of the corpus and publish or report the samples.
#[expect(
    clippy::too_many_lines,
    reason = "certification and the corpus loop are one ordered timeline; each project's measurement lives in the corpus part"
)]
pub(super) fn measure(
    command: &str,
    args: &mut dyn Iterator<Item = String>,
    observations: &mut dyn std::io::Write,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<(), Error> {
    if cfg!(debug_assertions) {
        return Err(Error::Invalid(constants::RELEASE_REQUIRED.into()));
    }
    let executable = PathBuf::from(
        args.next()
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let cache = PathBuf::from(
        args.next()
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_CACHE.into()))?,
    );
    let collector_path = PathBuf::from(
        args.next()
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_COLLECTOR.into()))?,
    )
    .canonicalize()?;
    std::fs::create_dir_all(&cache)?;
    let cache = cache.canonicalize()?;
    let mut tool = suite_source.profile(ArtifactPath(&commands::directory()))?;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let declared_inventory = tool.defaults.clone();
    let mut audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(&suite_source),
    )
    .audit(&commands::directory(), &tool)?;
    let authored_total = audit.authored_count();
    let selection_or_inventory = args.next();
    let scope = if matches!(
        command,
        constants::MEASURE | constants::MEASURE_ACCOUNTED_VALID
    ) {
        BenchmarkCoverage::Accounted
    } else {
        BenchmarkCoverage::Interim
    };
    let case_source = if scope == BenchmarkCoverage::Accounted {
        let input = selection_or_inventory
            .as_deref()
            .and_then(|s| s.strip_prefix(CASE_INVENTORY))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Invalid(CANONICAL_BENCHMARK_REQUIRES_EXPLI.into()))?;
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Registered {
            registration: std::path::PathBuf::from(input).canonicalize()?,
            output: tempfile::Builder::new()
                .prefix(REGISTERED_CASE_RESULTS)
                .tempdir_in(&cache)?
                .keep(),
        }
    } else {
        if let Some(selected) = selection_or_inventory {
            audit = audit.select(
                &selected
                    .split(',')
                    .map(ryo::linter::RuleId::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| Error::Invalid(e.to_string()))?,
            )?;
        }
        ryo_benchmark_contracts::domain::requests::types::CertificationCases::Profile
    };
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_MEASUREMENT_ARGUMENT.into()));
    }
    if scope == BenchmarkCoverage::Accounted {
        require_accounted(&audit, &tool)?;
    }
    let measurement_mode = if scope == BenchmarkCoverage::Accounted {
        ryo_benchmark_contracts::domain::measurements::types::MeasurementMode::Benchmark {
            inventory: declared_inventory,
        }
    } else {
        ryo_benchmark_contracts::domain::measurements::types::MeasurementMode::Diagnostic
    };
    ryo_benchmark::domain::measurements::service::require_inventory(
        &measurement_mode,
        audit.authored(),
        Some(audit.authored()),
    )?;
    let default_count = tool.defaults.len();
    let deferred_count = tool.deferred.len();
    let pending_count = tool.pending.len();
    let query_modules = tool.query_modules.clone();
    let rule_applicability = tool.rule_applicability.clone();
    let parser_configuration = tool.parser_configuration.clone();
    let source_language = tool.source_language.clone();
    let project_context_mode = tool.project_context_mode;
    let project_search_roots = tool.project_search_roots.clone();
    tool.defaults = audit.authored().to_vec();
    let candidate_defaults = tool.defaults.clone();
    let candidate_observations = tool.rule_observations.clone();
    let candidate_observation_selection = tool.rule_observation_selection;
    let declared_rule_context = tool.rule_context.clone();
    let configured_cohort = Some(tool.defaults.clone());
    let reference = Arc::new(commands::reference(executable.clone(), tool)?);
    let selected_scratch = tempfile::Builder::new()
        .prefix(constants::CERTIFICATION_PREFIX)
        .tempdir_in(&cache)?
        .keep();
    let selected_profile = selected_scratch.join(constants::PROFILE_DIRECTORY);
    ryo_benchmark::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(
        suite_source.as_ref(),
    )
    .stage_suite(
        &commands::directory(),
        &selected_profile,
        audit.authored(),
        ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Execution,
    )?;
    let frozen_workload = selected_scratch.join(constants::WORKLOAD_DIRECTORY);
    suite_source.stage_projects(
        ArtifactPath(&commands::directory().join(constants::LANGUAGE_DIRECTORY)),
        ArtifactPath(&frozen_workload),
    )?;
    let collector_settings =
        commands::collector(suite_source.as_ref(), &collector_path, &frozen_workload)?;
    let execution = commands::timed(collector_settings, observations)?;
    let selected_rules = selected_profile.join(constants::RULES_DIRECTORY);
    let candidate_executable = selected_scratch.join(constants::CANDIDATE_EXECUTABLE);
    let _bytes = std::fs::copy(std::env::current_exe()?, &candidate_executable)?;
    let candidate: Arc<dyn ryo_benchmark::outbound::linter::interface::LinterPort> =
        Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
            candidate_executable.clone(),
            selected_rules.clone(),
            query_modules,
            rule_applicability,
            parser_configuration,
            source_language,
            project_context_mode,
            candidate_defaults,
            candidate_observation_selection,
            candidate_observations,
        ));
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
    writeln!(
        observations,
        "scope: {}; {} verified / {default_count} shipped defaults; {deferred_count} assessed deferrals; {pending_count} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
        match scope {
            BenchmarkCoverage::Accounted => constants::ACCOUNTED,
            BenchmarkCoverage::Interim => constants::INTERIM,
        },
        audit.authored_count(),
        default_count - authored_total.0 - deferred_count - pending_count,
        authored_total.0 - audit.authored_count().0,
        certificate.cases(),
        certificate.fingerprint(),
        selected_profile.display()
    )?;
    output::environment(observations)?;
    let suite: Suite = suite_source.projects(ArtifactPath(&frozen_workload))?;
    if suite.language.0 != constants::PYTHON || !(2..=3).contains(&suite.projects.len()) {
        return Err(Error::Invalid(constants::PROJECT_COUNT_ERROR.into()));
    }
    let measurements = MeasureUseCaseImpl::new(
        Arc::<super::super::adapter::LinterRuffAdapter>::clone(&reference),
        Arc::<_>::clone(&candidate),
        Arc::<_>::clone(&execution),
        Arc::<_>::clone(&suite_source),
        measurement_mode,
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
        frozen_workload: &frozen_workload,
        suite_source: suite_source.as_ref(),
        reference: reference.as_ref(),
        candidate: candidate.as_ref(),
        execution: execution.as_ref(),
        certificate: &certificate,
        measurements: &measurements,
        comparison: &comparison,
        configured_cohort: &configured_cohort,
        declared_rule_context: &declared_rule_context,
        project_search_roots: &project_search_roots,
        project_context_mode,
    };
    let mut samples = Vec::new();
    let mut failures = Vec::new();
    for project in suite.projects {
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
        use ryo_benchmark::inbound::use_cases::publish::interface::PublishUseCase as _;
        ryo_benchmark::inbound::use_cases::publish::use_case::PublishUseCaseImpl::new(Arc::clone(
            &evidence,
        ))
        .publish(&samples)?;
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
