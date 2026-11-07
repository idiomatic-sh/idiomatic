//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use super::constants::{
    CASE_RECEIPT_DESTINATION_MUST_BE_N, CASE_SELECTION, CASE_SELECTION_MUST_BE_NONEMPTY_AN,
    DUPLICATE_CASE_SELECTION_ARGUMENT, DUPLICATE_CASE_SHARD_ARGUMENT, EXPECTED_SHARD_INDEX_COUNT,
    INVALID_CASE_SHARD_COUNT, INVALID_CASE_SHARD_INDEX, PINNED_REFERENCE_DOES_NOT_EVALUATE,
    PYTHON_TARGET, SHARD, UNEXPECTED_TRAILING_WORKER_ARGUMEN, UNEXPECTED_VERIFY_CASES_ARGUMENT,
    UNEXPECTED_WORKER_CONFIGURATION,
};
use super::diagnostic_protocol::WorkerTracing;
use super::request_input::{BenchmarkArgument, worker_input};
use super::targets::{ReferenceSpelling, TargetConfiguration};
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    stage_suite::interface::StageSuiteUseCase as _,
    verify_cases::interface::VerifyCasesUseCase as _,
    verify_reference_cases::interface::VerifyReferenceCasesUseCase as _,
};
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::{path::PathBuf, sync::Arc};

/// Execute one explicit child request, optionally writing a separately labelled trace.
pub(super) fn worker(
    mut args: impl Iterator<Item = BenchmarkArgument>,
    tracing: WorkerTracing,
) -> Result<(), Error> {
    let input = worker_input(&mut args)?;
    let next_argument = args.next().map(|argument| argument.0);
    let (parser_configuration, trace_path) = match next_argument {
        Some(configuration) if configuration.starts_with(PYTHON_TARGET) => {
            let trace_path = if tracing == WorkerTracing::Disabled {
                None
            } else {
                Some(
                    args.next()
                        .map(|argument| argument.0)
                        .ok_or_else(|| Error::Invalid(constants::MISSING_TRACE.into()))?,
                )
            };
            (Some(configuration), trace_path)
        }
        Some(path) if tracing != WorkerTracing::Disabled => (None, Some(path)),
        Some(_) => return Err(Error::Invalid(UNEXPECTED_WORKER_CONFIGURATION.into())),
        None if tracing != WorkerTracing::Disabled => {
            return Err(Error::Invalid(constants::MISSING_TRACE.into()));
        }
        None => (None, None),
    };
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_TRAILING_WORKER_ARGUMEN.into()));
    }
    super::request_input::checked_worker_identity(
        &input,
        parser_configuration.as_deref().map(TargetConfiguration),
    )?;
    let target_version = parser_target(parser_configuration.as_deref().map(TargetConfiguration))?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(target_version),
    )]);
    let frontend_diagnostic =
        ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from(constants::INVALID_SYNTAX)
                .map_err(|error| Error::Invalid(error.to_string()))?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        };
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort> =
        match (tracing, trace_path) {
            (WorkerTracing::Chrome, Some(path)) => Arc::new(
                ryo_benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                    languages,
                    Some(path.into()),
                    frontend_diagnostic.clone(),
                ),
            ),
            (WorkerTracing::JsonLines, Some(path)) => Arc::new(
                ryo_benchmark::outbound::engine::factory::local_with_diagnostic_events(
                    languages,
                    path.into(),
                    frontend_diagnostic,
                ),
            ),
            (WorkerTracing::Disabled, None) => Arc::new(
                ryo_benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                    languages,
                    None,
                    frontend_diagnostic,
                ),
            ),
            _ => return Err(Error::Invalid(constants::MISSING_TRACE.into())),
        };
    let transport =
        ryo_benchmark::outbound::worker::factory::json(input.0.into(), Box::new(std::io::stdout()));
    RunRyoUseCaseImpl::new(engine, Arc::new(transport)).run_ryo()
}

/// Verify pinned tools, complete rule accounting and independently authored findings.
pub(super) fn verify_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<CaseCount, Error> {
    let execution = Arc::new(ryo_benchmark::outbound::execution::factory::process());
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    let mut case_receipts = None;
    let mut shard = None;
    let mut case_selection = None;
    for argument in args {
        if let Some(path) = argument
            .0
            .strip_prefix(ryo_benchmark::outbound::inventory::types::CASE_RECEIPTS_ARGUMENT)
        {
            if path.is_empty() || case_receipts.replace(PathBuf::from(path)).is_some() {
                return Err(Error::Invalid(CASE_RECEIPT_DESTINATION_MUST_BE_N.into()));
            }
        } else if argument.0.starts_with(SHARD) {
            if shard
                .replace(parse_shard(ShardCoordinates(&argument.0))?)
                .is_some()
            {
                return Err(Error::Invalid(DUPLICATE_CASE_SHARD_ARGUMENT.into()));
            }
        } else if let Some(path) = argument.0.strip_prefix(CASE_SELECTION) {
            if case_selection.is_some() {
                return Err(Error::Invalid(DUPLICATE_CASE_SELECTION_ARGUMENT.into()));
            }
            let selected = ryo_benchmark::outbound::suite::factory::local()
                .selection(ArtifactPath(std::path::Path::new(path)))?;
            let selected_count = selected.len();
            let selected = selected
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            if selected.is_empty() || selected.len() != selected_count {
                return Err(Error::Invalid(CASE_SELECTION_MUST_BE_NONEMPTY_AN.into()));
            }
            case_selection = Some(selected);
        } else {
            return Err(Error::Invalid(UNEXPECTED_VERIFY_CASES_ARGUMENT.into()));
        }
    }
    let suite_source: Arc<dyn SuitePort> = match case_receipts {
        Some(path) => {
            Arc::new(ryo_benchmark::outbound::suite::factory::local_with_case_receipts(path))
        }
        None => Arc::new(ryo_benchmark::outbound::suite::factory::local()),
    };
    let mut tool = suite_source.profile(ArtifactPath(&source))?;
    let query_modules = tool.query_modules.clone();
    let rule_applicability = tool.rule_applicability.clone();
    let parser_configuration = tool.parser_configuration.clone();
    let source_language = tool.source_language.clone();
    let project_context_mode = tool.project_context_mode;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(&suite_source),
    )
    .audit(&source, &tool)?;
    ryo_benchmark::domain::suite::service::require_accounted(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let candidate_defaults = tool.defaults.clone();
    let candidate_observations = tool.rule_observations.clone();
    let candidate_observation_selection = tool.rule_observation_selection;
    let reference = Arc::new(reference(executable, tool)?);
    let scratch = suite_source.scratch(
        ArtifactPath(std::path::Path::new(constants::DEFAULT_SCRATCH_ROOT)),
        constants::VERIFY_CASES_SCRATCH,
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(&scratch))?;
    let profile = root.join(constants::PROFILE_DIRECTORY);
    ryo_benchmark::inbound::use_cases::stage_suite::use_case::StageSuiteUseCaseImpl::new(
        suite_source.as_ref(),
    )
    .stage_suite(
        &source,
        &profile,
        audit.authored(),
        ryo_benchmark_contracts::domain::workloads::types::StagePurpose::Certification,
    )?;
    let candidate = Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
        std::env::current_exe()?,
        profile.join(crate::constants::RULES_DIRECTORY),
        query_modules,
        rule_applicability,
        parser_configuration,
        source_language,
        project_context_mode,
        candidate_defaults,
        candidate_observation_selection,
        candidate_observations,
    ));
    let count =
        ryo_benchmark::inbound::use_cases::verify_cases::use_case::VerifyCasesUseCaseImpl::new(
            suite_source.as_ref(),
            reference.as_ref(),
            candidate.as_ref(),
            execution.as_ref(),
            &ryo_benchmark::outbound::inventory::factory::local(),
        )
        .verify_cases(
            &profile.join(crate::constants::RULES_DIRECTORY),
            &root.join(constants::CASES_DIRECTORY),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection {
                shard,
                ids: case_selection.as_ref(),
            },
            ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
        );
    suite_source.discard(ArtifactPath(&scratch))?;
    count
}

/// The text of one `--shard=index/count` argument.
struct ShardCoordinates<'a>(&'a str);

/// Parse explicit zero-based shard coordinates using the validated application type.
fn parse_shard(
    value: ShardCoordinates<'_>,
) -> Result<ryo_benchmark_contracts::domain::requests::types::case_shard::value::CaseShard, Error> {
    let coordinates = value
        .0
        .strip_prefix(SHARD)
        .ok_or_else(|| Error::Invalid(EXPECTED_SHARD_INDEX_COUNT.into()))?;
    let (index, count) = coordinates
        .split_once('/')
        .ok_or_else(|| Error::Invalid(EXPECTED_SHARD_INDEX_COUNT.into()))?;
    let index = index
        .parse()
        .map_err(|_invalid_index| Error::Invalid(INVALID_CASE_SHARD_INDEX.into()))?;
    let count = count
        .parse()
        .map_err(|_invalid_count| Error::Invalid(INVALID_CASE_SHARD_COUNT.into()))?;
    ryo_benchmark_contracts::domain::requests::types::case_shard::value::CaseShard::new(
        index, count,
    )
}

/// Validate authored fixture expectations against Ruff without loading a Ryo candidate.
pub(super) fn verify_reference_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, Error>{
    let suite_source = ryo_benchmark::outbound::suite::factory::local();
    let execution = ryo_benchmark::outbound::execution::factory::process();
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    let tool = suite_source.profile(ArtifactPath(&source))?;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let reference = reference(executable, tool)?;
    let scratch = suite_source.scratch(
        ArtifactPath(std::path::Path::new(constants::DEFAULT_SCRATCH_ROOT)),
        constants::VERIFY_REFERENCE_SCRATCH,
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(&scratch))?;
    let count = ryo_benchmark::inbound::use_cases::verify_reference_cases::use_case::VerifyReferenceCasesUseCaseImpl::new(&suite_source, &reference, &execution, &ryo_benchmark::outbound::inventory::factory::local()).verify_reference_cases(
        &source.join(crate::constants::RULES_DIRECTORY),
        &root,
    );
    suite_source.discard(ArtifactPath(&scratch))?;
    count
}

/// Compose the selected reference rule subset with its verified executable.
pub(super) fn reference(executable: PathBuf, tool: Tool) -> Result<LinterRuffAdapter, Error> {
    if tool.file_observation.is_some()
        && super::super::host::permission_checks_supported()
            == super::super::host_policy::Permissions::Unsupported
    {
        return Err(Error::Invalid(PINNED_REFERENCE_DOES_NOT_EVALUATE.into()));
    }
    let target_version = ruff_target(
        tool.parser_configuration
            .as_deref()
            .map(TargetConfiguration),
    )?
    .0
    .to_owned();
    Ok(LinterRuffAdapter::new(LinterRuffSettings {
        executable,
        defaults: tool.defaults.into_iter().collect(),
        target_version: target_version.into(),
    }))
}

/// Map the sealed shared identity to Ruff's explicit CLI value.
fn ruff_target(configuration: Option<TargetConfiguration<'_>>) -> Result<ReferenceSpelling, Error> {
    Ok(super::targets::resolve(configuration)?.reference)
}

/// Use exactly the same explicit identity for the linked parser as the reference.
fn parser_target(
    configuration: Option<TargetConfiguration<'_>>,
) -> Result<ryo_python::settings::PythonTargetVersion, Error> {
    Ok(super::targets::resolve(configuration)?.parser)
}

/// Locate this language-owned linter profile from the Cargo manifest.
pub(super) fn directory() -> PathBuf {
    std::path::Path::new(constants::MANIFEST_DIRECTORY).join(constants::RUFF_DIRECTORY)
}

/// Copy exact collector bytes into the frozen workload and record their digest.
pub(super) fn collector(
    suite: &dyn SuitePort,
    source: &std::path::Path,
    workload: &std::path::Path,
) -> Result<CollectorIdentity, Error> {
    let executable = workload.join(constants::COLLECTOR_EXECUTABLE);
    suite.copy_file(ArtifactPath(source), ArtifactPath(&executable))?;
    let digest = suite.digest(ArtifactPath(&executable))?;
    suite.write_text(
        ArtifactPath(&workload.join(constants::COLLECTOR_IDENTITY)),
        &digest.0,
    )?;
    Ok(CollectorIdentity { executable, digest })
}

/// Copied collector identity; certification subsequently seals these artifacts.
pub(super) struct CollectorIdentity {
    /// Frozen native collector path.
    pub executable: PathBuf,
    /// Digest computed from the copied bytes.
    pub digest: ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
}

/// Compose the isolated collector and expose its exact identity in observations.
pub(super) fn timed(
    identity: CollectorIdentity,
    observations: &mut dyn std::io::Write,
) -> Result<Arc<dyn ryo_benchmark::outbound::execution::interface::ExecutionPort>, Error> {
    writeln!(
        observations,
        "collector: executable={} sha256={}",
        identity.executable.display(),
        identity.digest
    )?;
    Ok(Arc::new(
        ryo_benchmark::outbound::execution::factory::timed(identity.executable, identity.digest),
    ))
}

/// Report authored coverage while refusing unaccounted shipped defaults.
pub(super) fn audit(
    suite_source: &Arc<dyn SuitePort>,
    output: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let tool = suite_source.profile(ArtifactPath(&directory()))?;
    let result = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(suite_source),
    )
    .audit(&directory(), &tool)?;
    writeln!(
        output,
        "{} {}: {} queries authored / {} shipped defaults (not a parity certificate)",
        tool.tool,
        tool.version,
        result.authored_count(),
        tool.defaults.len()
    )?;
    ryo_benchmark::domain::suite::service::require_accounted(&result, &tool)
}
