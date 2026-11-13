//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterOxlintAdapter, settings::LinterOxlintSettings};
use super::constants;
use super::constants::UNEXPECTED_TRAILING_WORKER_ARGUMENT;
use super::diagnostic_protocol::WorkerTracing;
use super::locations::{default_scratch, directory};
use super::request_input::{BenchmarkArgument, WorkerInput, worker_input};
use ryo_benchmark::inbound::use_cases::account_defaults::{
    interface::AccountDefaultsUseCase as _, use_case::AccountDefaultsUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::audit::interface::AuditUseCase as _;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    select_cases::{interface::SelectCasesUseCase as _, types::CaseArgument},
    stage_suite::interface::StageSuiteUseCase as _,
    verify_cases::interface::VerifyCasesUseCase as _,
    verify_reference_cases::interface::VerifyReferenceCasesUseCase as _,
};
use ryo_benchmark::outbound::linter::interface::LinterPort as _;
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
    let trace_path = match (tracing, args.next().map(|argument| argument.0)) {
        (WorkerTracing::Disabled, None) => None,
        (WorkerTracing::Disabled, Some(_)) => {
            return Err(Error::Invalid(UNEXPECTED_TRAILING_WORKER_ARGUMENT.into()));
        }
        (WorkerTracing::Chrome | WorkerTracing::JsonLines, Some(path)) => Some(path),
        (WorkerTracing::Chrome | WorkerTracing::JsonLines, None) => {
            return Err(Error::Invalid(constants::MISSING_TRACE.into()));
        }
    };
    if args.next().is_some() {
        return Err(Error::Invalid(UNEXPECTED_TRAILING_WORKER_ARGUMENT.into()));
    }
    let source = match input {
        WorkerInput::File(path) => WorkerSource::Stored(path),
        WorkerInput::Inline(payload) => WorkerSource::Argument(payload.0.into()),
    };
    let transport: Arc<dyn WorkerPort> = Arc::new(ryo_benchmark::outbound::worker::factory::json(
        source,
        Box::new(std::io::stdout()),
    ));
    let languages = ryo::linter::Languages(vec![ryo_typescript::linked()]);
    let frontend_diagnostic =
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from(crate::constants::INVALID_SYNTAX)
                .map_err(|error| Error::Invalid(error.to_string()))?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        };
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort> =
        match (tracing, trace_path) {
            (WorkerTracing::JsonLines, Some(path)) => Arc::new(
                ryo_benchmark::outbound::engine::factory::local_with_diagnostic_events(
                    languages,
                    path.into(),
                    frontend_diagnostic,
                ),
            ),
            (_, path) => Arc::new(
                ryo_benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                    languages,
                    path.map(Into::into),
                    frontend_diagnostic,
                ),
            ),
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
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_OXLINT.into()))?,
    )
    .canonicalize()?;
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    let flags = args
        .map(|argument| CaseArgument(argument.0))
        .collect::<Vec<_>>();
    let run =
        ryo_benchmark::inbound::use_cases::select_cases::use_case::SelectCasesUseCaseImpl::new()
            .select_cases(&flags)?;
    let suite_source: Arc<dyn SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let mut tool = suite_source.profile(ArtifactPath(&source))?;
    let query_modules = tool.query_modules.clone();
    let rule_applicability = tool.rule_applicability.clone();
    let parser_configuration = tool.parser_configuration.clone();
    let source_language = tool.source_language.clone();
    let project_context_mode = tool.project_context_mode;
    let expected = reference(executable.clone(), tool.clone()).version_line(&tool);
    suite_source.verify_tool(ArtifactPath(&executable), &tool, &expected)?;
    let audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(&suite_source),
    )
    .audit(&source, &tool)?;
    AccountDefaultsUseCaseImpl::new().account_defaults(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let candidate_defaults = tool.defaults.clone();
    let candidate_observations = tool.rule_observations.clone();
    let candidate_observation_selection = tool.rule_observation_selection;
    let reference = Arc::new(reference(executable, tool));
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
            ryo_benchmark_contracts::domain::requests::types::CaseSelection { shard: run.shard },
            ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
        );
    drop(scratch);
    count
}

/// Validate authored fixture expectations against oxlint without loading a Ryo candidate.
pub(super) fn verify_reference_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<ryo_benchmark_contracts::domain::workloads::types::reference_case_count::value::ReferenceCaseCount, Error>{
    let suite_source = ryo_benchmark::outbound::suite::factory::local();
    let execution = ryo_benchmark::outbound::execution::factory::process();
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_OXLINT.into()))?,
    )
    .canonicalize()?;
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    let tool = suite_source.profile(ArtifactPath(&source))?;
    let reference = reference(executable.clone(), tool.clone());
    suite_source.verify_tool(
        ArtifactPath(&executable),
        &tool,
        &reference.version_line(&tool),
    )?;
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
pub(super) fn reference(executable: PathBuf, tool: Tool) -> LinterOxlintAdapter {
    LinterOxlintAdapter::new(LinterOxlintSettings {
        executable,
        defaults: tool.defaults.into_iter().collect(),
    })
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
        ArtifactText(&digest.0),
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
