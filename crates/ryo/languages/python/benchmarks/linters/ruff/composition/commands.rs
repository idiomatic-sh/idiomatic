//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use super::constants::{
    PINNED_REFERENCE_DOES_NOT_EVALUATE, PYTHON_TARGET, UNEXPECTED_TRAILING_WORKER_ARGUMENT,
    UNEXPECTED_WORKER_CONFIGURATION,
};
use super::diagnostic_protocol::WorkerTracing;
use super::locations::{default_scratch, directory};
use super::request_input::{BenchmarkArgument, WorkerInput, worker_expectation, worker_input};
use super::targets::{ReferenceSpelling, TargetConfiguration};
use ryo_benchmark::inbound::use_cases::account_defaults::{
    interface::AccountDefaultsUseCase as _, use_case::AccountDefaultsUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::check_worker_request::{
    interface::CheckWorkerRequestUseCase as _, use_case::CheckWorkerRequestUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    select_cases::{
        interface::SelectCasesUseCase as _,
        types::{CaseArgument, SelectedIds},
    },
    stage_suite::interface::StageSuiteUseCase as _,
    verify_cases::interface::VerifyCasesUseCase as _,
    verify_reference_cases::interface::VerifyReferenceCasesUseCase as _,
};
use ryo_benchmark::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark::outbound::worker::{interface::WorkerPort, types::WorkerSource};
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
        return Err(Error::Invalid(UNEXPECTED_TRAILING_WORKER_ARGUMENT.into()));
    }
    let expectation = worker_expectation(parser_configuration.as_deref().map(TargetConfiguration))?;
    let source = match input {
        WorkerInput::File(path) => WorkerSource::Stored(path),
        WorkerInput::Inline(payload) => WorkerSource::Argument(payload.0.into()),
    };
    let transport: Arc<dyn WorkerPort> = Arc::new(ryo_benchmark::outbound::worker::factory::json(
        source,
        Box::new(std::io::stdout()),
    ));
    CheckWorkerRequestUseCaseImpl::new(Arc::clone(&transport))
        .check_worker_request(&expectation)?;
    let target_version = parser_target(parser_configuration.as_deref().map(TargetConfiguration))?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(target_version),
    )]);
    let frontend_diagnostic =
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
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
    RunRyoUseCaseImpl::new(
        engine,
        transport,
        Arc::new(ryo_benchmark::outbound::context::factory::local()),
    )
    .run_ryo()
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
    let flags = args
        .map(|argument| CaseArgument(argument.0))
        .collect::<Vec<_>>();
    let run =
        ryo_benchmark::inbound::use_cases::select_cases::use_case::SelectCasesUseCaseImpl::new(
            Arc::new(ryo_benchmark::outbound::suite::factory::local()),
        )
        .select_cases(&flags)?;
    let suite_source: Arc<dyn SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
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
    AccountDefaultsUseCaseImpl::new().account_defaults(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let candidate_defaults = tool.defaults.clone();
    let candidate_observations = tool.rule_observations.clone();
    let candidate_observation_selection = tool.rule_observation_selection;
    let reference = Arc::new(reference(executable, tool)?);
    let scratch = default_scratch(
        suite_source.as_ref(),
        ArtifactText(constants::VERIFY_CASES_SCRATCH),
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(scratch.path()))?;
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
        profile.join(constants::RULES_DIRECTORY),
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
            &ryo_benchmark::outbound::receipts::factory::local(run.receipts),
            &ryo_benchmark::outbound::context::factory::local(),
            reference.as_ref(),
            candidate.as_ref(),
            execution.as_ref(),
            &ryo_benchmark::outbound::inventory::factory::local(),
        )
        .verify_cases(
            &profile.join(constants::RULES_DIRECTORY),
            &root.join(constants::CASES_DIRECTORY),
            ryo_benchmark_contracts::domain::requests::types::CaseSelection {
                shard: run.shard,
                ids: match &run.ids {
                    SelectedIds::WholeShard => {
                        ryo_benchmark_contracts::domain::requests::types::IdSelection::WholeShard
                    }
                    SelectedIds::Subset(ids) => {
                        ryo_benchmark_contracts::domain::requests::types::IdSelection::Subset(ids)
                    }
                },
            },
            ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
        );
    drop(scratch);
    count
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
    let scratch = default_scratch(
        &suite_source,
        ArtifactText(constants::VERIFY_REFERENCE_SCRATCH),
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(scratch.path()))?;
    let count = ryo_benchmark::inbound::use_cases::verify_reference_cases::use_case::VerifyReferenceCasesUseCaseImpl::new(&suite_source, &ryo_benchmark::outbound::receipts::factory::local(ryo_benchmark::outbound::receipts::types::CaseReceipts::Unrecorded), &ryo_benchmark::outbound::context::factory::local(), &reference, &execution, &ryo_benchmark::outbound::inventory::factory::local()).verify_reference_cases(
        &source.join(constants::RULES_DIRECTORY),
        &root,
    );
    drop(scratch);
    count
}

/// Compose the selected reference rule subset with its verified executable.
pub(super) fn reference(executable: PathBuf, tool: Tool) -> Result<LinterRuffAdapter, Error> {
    if tool.file_observation
        != ryo_benchmark_contracts::domain::workloads::types::FileObservation::Undeclared
        && crate::host::permission_checks_supported()
            == super::super::host_policy::Permissions::Unsupported
    {
        return Err(Error::Invalid(PINNED_REFERENCE_DOES_NOT_EVALUATE.into()));
    }
    let target_version = ruff_target(match &tool.parser_configuration {
        ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(name) => {
            Some(TargetConfiguration(name.0.as_str()))
        }
        ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified => None,
    })?
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
        ryo_benchmark::outbound::suite::types::ArtifactText(&digest.0),
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
    AccountDefaultsUseCaseImpl::new().account_defaults(&result, &tool)
}
