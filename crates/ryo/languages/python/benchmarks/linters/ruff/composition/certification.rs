//! Certification of a measure invocation: what it names, what it stages, which contestant it builds and what it reports.
use super::constants;
use super::constants::{
    CANONICAL_BENCHMARK_REQUIRES_EXPLI, CASE_INVENTORY, REGISTERED_CASE_RESULTS,
};
use super::locations;
use ryo_benchmark::domain::command_line::types::{BenchmarkArgument, Selection};
use ryo_benchmark::domain::suite::types::audit::value::Audit;
use ryo_benchmark::inbound::use_cases::stage_suite::{
    interface::StageSuiteUseCase as _, use_case::StageSuiteUseCaseImpl,
};
use ryo_benchmark::outbound::suite::interface::SuitePort;
use ryo_benchmark::outbound::suite::types::{ArtifactPath, ArtifactText};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{BenchmarkCoverage, CertificationCases};
use ryo_benchmark_contracts::domain::workloads::types::{StagePurpose, Tool};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
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
            .retained_directory(ArtifactPath(cache), ArtifactText(REGISTERED_CASE_RESULTS))?
            .0,
    })
}

/// The directories one certification stages: its scratch root, the suite profile and the frozen workload.
pub(super) struct StagedSuite {
    pub(super) scratch: PathBuf,
    pub(super) profile: PathBuf,
    pub(super) workload: PathBuf,
}

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
/// Stage the suite of the authored rules and its projects into a fresh scratch directory under the cache.
pub(super) fn stage(
    suite_source: &Arc<dyn SuitePort>,
    cache: &Path,
    authored: &[ryo::linter::RuleId],
) -> Result<StagedSuite, Error> {
    let scratch = suite_source
        .retained_directory(
            ArtifactPath(cache),
            ArtifactText(constants::CERTIFICATION_PREFIX),
        )?
        .0;
    let profile = scratch.join(constants::PROFILE_DIRECTORY);
    StageSuiteUseCaseImpl::new(suite_source.as_ref()).stage_suite(
        &locations::directory(),
        &profile,
        authored,
        StagePurpose::Execution,
    )?;
    let workload = scratch.join(constants::WORKLOAD_DIRECTORY);
    suite_source.stage_projects(
        ArtifactPath(&locations::directory().join(constants::LANGUAGE_DIRECTORY)),
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

/// The audit a run certifies, and the cases it is certified against.
pub(super) struct Certifying {
    pub(super) audit: Audit,
    pub(super) cases: CertificationCases,
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
