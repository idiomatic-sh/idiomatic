//! Certification of a measure invocation: what it names, what it stages, which contestant it builds and what it reports.
use super::commands;
use super::constants;
use super::constants::{
    CANONICAL_BENCHMARK_REQUIRES_EXPLI, CASE_INVENTORY, REGISTERED_CASE_RESULTS,
};
use super::request_input::BenchmarkArgument;
use ryo_benchmark::domain::suite::service::require_accounted;
use ryo_benchmark::domain::suite::types::audit::value::Audit;
use ryo_benchmark::domain::suite::types::certificate::value::Certificate;
use ryo_benchmark::inbound::use_cases::stage_suite::{
    interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl,
};
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::{ArtifactPath, ArtifactText};
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{BenchmarkCoverage, CertificationCases};
use ryo_benchmark_contracts::domain::workloads::types::rule_count::value::RuleCount;
use ryo_benchmark_contracts::domain::workloads::types::{StagePurpose, Tool};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// The three executables and the cache directory a measure invocation names, in the order it names them.
pub(super) struct MeasurePaths {
    pub(super) executable: PathBuf,
    pub(super) cache: PathBuf,
    pub(super) collector_path: PathBuf,
}

/// Read the reference executable, the cache directory and the collector from the arguments.
pub(super) fn paths(
    args: &mut dyn Iterator<Item = BenchmarkArgument>,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<MeasurePaths, Error> {
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
    let cache = suite_source.ensure_directory(ArtifactPath(&cache))?;
    Ok(MeasurePaths {
        executable,
        cache,
        collector_path,
    })
}

/// Make a scratch directory under the suite boundary's default location.
pub(super) fn scratch(suite: &dyn SuitePort, prefix: &str) -> Result<PathBuf, Error> {
    suite
        .scratch(
            ArtifactPath(std::path::Path::new(constants::DEFAULT_SCRATCH_ROOT)),
            ArtifactText(prefix),
        )
        .map(|directory| directory.0)
}

/// The registered case inventory an accounted measurement is held to, with the scratch directory for its results.
fn registered_cases(
    selection_or_inventory: Option<Selection<'_>>,
    cache: &Path,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<CertificationCases, Error> {
    let input = selection_or_inventory
        .and_then(|s| s.0.strip_prefix(CASE_INVENTORY))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Error::Invalid(CANONICAL_BENCHMARK_REQUIRES_EXPLI.into()))?;
    Ok(CertificationCases::Registered {
        registration: PathBuf::from(input).canonicalize()?,
        output: suite_source
            .scratch(ArtifactPath(cache), ArtifactText(REGISTERED_CASE_RESULTS))?
            .0,
    })
}

/// The directories one certification stages: its scratch root, the suite profile and the frozen workload.
pub(super) struct StagedSuite {
    pub(super) scratch: PathBuf,
    pub(super) profile: PathBuf,
    pub(super) workload: PathBuf,
}

/// Stage the suite of the authored rules and its projects into a fresh scratch directory under the cache.
pub(super) fn stage(
    suite_source: &Arc<dyn SuitePort>,
    cache: &Path,
    authored: &[ryo::linter::RuleId],
) -> Result<StagedSuite, Error> {
    let scratch = suite_source
        .scratch(ArtifactPath(cache), ArtifactText(constants::CERTIFICATION_PREFIX))?
        .0;
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
    Ok(StagedSuite {
        scratch,
        profile,
        workload,
    })
}

/// The Ryo contestant: this executable bound to the staged rules and to the settings the reference tool declares.
pub(super) fn candidate_linter(
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

/// Whether a measure command holds the run to the accounted inventory or only to the rules it selects.
pub(super) fn scope_of(command: MeasureCommand<'_>) -> BenchmarkCoverage {
    if matches!(
        command.0,
        constants::MEASURE | constants::MEASURE_ACCOUNTED_VALID
    ) {
        BenchmarkCoverage::Accounted
    } else {
        BenchmarkCoverage::Interim
    }
}

/// Narrow the audit to the rules the invocation selects, and certify against the profile's own cases.
fn profile_cases(audit: Audit, selection: Option<Selection<'_>>) -> Result<Certifying, Error> {
    let audit = match selection {
        Some(selected) => audit.select(
            &selected
                .0
                .split(',')
                .map(ryo::linter::RuleId::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| Error::Invalid(e.to_string()))?,
        )?,
        None => audit,
    };
    Ok(Certifying {
        audit,
        cases: CertificationCases::Profile,
    })
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
pub(super) struct RuleTally {
    defaults: Counted,
    deferred: Counted,
    pending: Counted,
    authored: RuleCount,
}

/// A number of rules of one kind.
#[derive(Clone, Copy, Debug)]
pub(super) struct Counted(usize);

/// The first argument of a measure invocation.
#[derive(Clone, Copy, Debug)]
pub(super) struct MeasureCommand<'a>(pub(super) &'a str);

/// What a measure invocation selected: a comma list of rules, or the registered case inventory.
#[derive(Clone, Copy, Debug)]
pub(super) struct Selection<'a>(pub(super) &'a str);

/// The audit a run certifies, and the cases it is certified against.
pub(super) struct Certifying {
    pub(super) audit: Audit,
    pub(super) cases: CertificationCases,
}

/// Say what the certification verified, and what it left unassessed.
pub(super) fn announce(
    observations: &mut dyn std::io::Write,
    scope: BenchmarkCoverage,
    tally: &RuleTally,
    audit: &Audit,
    certificate: &Certificate,
    profile: &Path,
) -> Result<(), Error> {
    let RuleTally {
        defaults,
        deferred,
        pending,
        authored,
    } = tally;
    writeln!(
        observations,
        "scope: {}; {} verified / {} shipped defaults; {} assessed deferrals; {} assessed pending; {} unassessed; {} authored outside subset; {} cases; certificate={}; artifacts={}",
        match scope {
            BenchmarkCoverage::Accounted => constants::ACCOUNTED,
            BenchmarkCoverage::Interim => constants::INTERIM,
        },
        audit.authored_count(),
        defaults.0,
        deferred.0,
        pending.0,
        defaults.0 - authored.0 - deferred.0 - pending.0,
        authored.0 - audit.authored_count().0,
        certificate.cases(),
        certificate.fingerprint(),
        profile.display()
    )?;
    Ok(())
}

/// The mode the run is measured in, after the accounted run has shown its inventory and every run its authored rules.
pub(super) fn held_to(
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

impl RuleTally {
    /// Count the tool's defaults, deferrals and pending rules beside the authored total.
    pub(super) fn of(tool: &Tool, authored: RuleCount) -> Self {
        Self {
            defaults: Counted(tool.defaults.len()),
            deferred: Counted(tool.deferred.len()),
            pending: Counted(tool.pending.len()),
            authored,
        }
    }
}

/// The cases the run is certified against, and the audit narrowed to what it selects.
pub(super) fn cases_for(
    scope: BenchmarkCoverage,
    audit: Audit,
    selection_or_inventory: Option<Selection<'_>>,
    cache: &Path,
    suite_source: &Arc<dyn SuitePort>,
) -> Result<Certifying, Error> {
    if scope == BenchmarkCoverage::Accounted {
        Ok(Certifying {
            audit,
            cases: registered_cases(selection_or_inventory, cache, suite_source)?,
        })
    } else {
        profile_cases(audit, selection_or_inventory)
    }
}
