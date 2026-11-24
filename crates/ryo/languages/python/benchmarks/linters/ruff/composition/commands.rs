//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use super::constants::{
    PYTHON_TARGET, UNEXPECTED_TRAILING_WORKER_ARGUMENT, UNEXPECTED_WORKER_CONFIGURATION,
};
use super::locations::{default_scratch, directory};
use super::request_input::worker_expectation;
use super::targets::{ReferenceSpelling, TargetConfiguration};
use ryo_benchmark::domain::command_line::types::{
    BenchmarkArgument, WorkerTracing, worker_input::value::WorkerInput,
};
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
use ryo_benchmark::inbound::use_cases::select_cases::types::{CaseRun, VerifyTimes};
use ryo_benchmark::inbound::use_cases::verify_rule_profiles::{
    interface::VerifyRuleProfilesUseCase as _, use_case::VerifyRuleProfilesUseCaseImpl,
};
use ryo_benchmark::inbound::use_cases::{
    select_cases::{interface::SelectCasesUseCase as _, types::CaseArgument},
    stage_suite::interface::StageSuiteUseCase as _,
    verify_cases::interface::VerifyCasesUseCase as _,
};
use ryo_benchmark::outbound::suite::{
    interface::SuitePort,
    types::{ArtifactPath, ArtifactText},
};
use ryo_benchmark::outbound::worker::{interface::WorkerPort, types::WorkerSource};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::Tool;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// Execute one explicit child request, optionally writing a separately labelled trace.
pub(super) fn worker(
    mut args: impl Iterator<Item = BenchmarkArgument>,
    tracing: WorkerTracing,
) -> Result<(), Error> {
    let input = WorkerInput::read(&mut args)?;
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
    let transport: Arc<dyn WorkerPort + Send + Sync> = Arc::new(
        ryo_benchmark::outbound::worker::factory::json(source, Box::new(std::io::stdout())),
    );
    CheckWorkerRequestUseCaseImpl::new(Arc::<dyn WorkerPort + Send + Sync>::clone(&transport))
        .check_worker_request(&expectation)?;
    let target_version = parser_target(parser_configuration.as_deref().map(TargetConfiguration))?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(target_version),
    )]);
    let frontend_diagnostic = frontend_diagnostic()?;
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort + Send + Sync> =
        match (tracing, trace_path) {
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
        ryo_benchmark::inbound::use_cases::select_cases::use_case::SelectCasesUseCaseImpl::new()
            .select_cases(&flags)?;
    // Each rule is verified under its own settings profile, reached through the harness's one registrar.
    let suite = ryo_benchmark::outbound::suite::factory::local();
    VerifyRuleProfilesUseCaseImpl::new(
        &suite,
        &ryo_benchmark::outbound::inventory::factory::local(),
    )
    .verify_rule_profiles(
        &source,
        run.shard,
        ArtifactText(constants::VERIFY_CASES_SCRATCH),
        &mut |profile| verify_profile(&executable, profile, &run),
    )
}

/// Verify the profile at `source` against both contestants, its rules under its one settings profile.
fn verify_profile(executable: &Path, source: &Path, run: &CaseRun) -> Result<CaseCount, Error> {
    let executable = executable.to_owned();
    let source = source.to_owned();
    let suite_source: Arc<dyn SuitePort> =
        Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let tool = suite_source.profile(ArtifactPath(&source))?;
    // The bench verified the reference tool's pinned bytes and version line once, before any rule (`verify-tool`); the
    // registration pins its digest, which the inventory checks before and after every rule.
    let audit = ryo_benchmark::inbound::use_cases::audit::use_case::AuditUseCaseImpl::new(
        Arc::clone(&suite_source),
    )
    .audit(&source, &tool)?;
    AccountDefaultsUseCaseImpl::new().account_defaults(&audit, &tool)?;
    let scratch = default_scratch(
        suite_source.as_ref(),
        ArtifactText(constants::VERIFY_CASES_SCRATCH),
    )?;
    let root = suite_source.ensure_directory(ArtifactPath(scratch.path()))?;
    let profile = root.join(constants::PROFILE_DIRECTORY);
    let staged =
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
        staged.query_modules.clone(),
        staged.rule_applicability.clone(),
        staged.parser_configuration.clone(),
        staged.source_language.clone(),
        staged.project_context_mode,
        staged.defaults.clone(),
        staged.rule_observation_selection,
        staged.rule_observations.clone(),
    ));
    let target_version = parser_target(configured(&staged.parser_configuration))?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(target_version),
    )]);
    let frontend_diagnostic = frontend_diagnostic()?;
    // The candidate runs in this process over one engine, which compiles each rule once and keeps its plans for every case.
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort + Send + Sync> =
        Arc::new(
            ryo_benchmark::outbound::engine::factory::retaining_with_frontend_diagnostic(
                languages,
                frontend_diagnostic,
            ),
        );
    let context: Arc<dyn ryo_benchmark::outbound::context::interface::ContextPort + Send + Sync> =
        Arc::new(ryo_benchmark::outbound::context::factory::local());
    // The worker in this process reads each stored request it is handed and answers its bytes through the JSON transport, which
    // receives and publishes nothing of its own.
    let transport = Arc::new(ryo_benchmark::outbound::worker::factory::json(
        ryo_benchmark::outbound::worker::types::WorkerSource::Argument(std::ffi::OsString::new()),
        Box::new(std::io::sink()),
    ));
    let worker = RunRyoUseCaseImpl::new(engine, transport, context);
    let execution = ryo_benchmark::outbound::execution::factory::in_process(&worker);
    // The times of a verification go to its times file when the run names one, and nowhere when it does not.
    let evidence: Box<dyn ryo_benchmark::outbound::evidence::interface::EvidencePort> =
        match &run.timings {
            VerifyTimes::Recorded(path) => Box::new(
                ryo_benchmark::outbound::evidence::factory::bmf_appending(path.clone()),
            ),
            VerifyTimes::Unrecorded => Box::new(ryo_benchmark::outbound::evidence::factory::bmf(
                Box::new(std::io::sink()),
                Box::new(std::io::sink()),
            )),
        };
    let reference = Arc::new(reference(executable, staged)?);
    let count =
        ryo_benchmark::inbound::use_cases::verify_cases::use_case::VerifyCasesUseCaseImpl::new(
            suite_source.as_ref(),
            &ryo_benchmark::outbound::host::factory::local(),
            &ryo_benchmark::outbound::receipts::factory::local(run.receipts.clone()),
            &ryo_benchmark::outbound::context::factory::local(),
            reference.as_ref(),
            candidate.as_ref(),
            ryo_benchmark::inbound::use_cases::verify_cases::types::Executions {
                reference: &ryo_benchmark::outbound::execution::factory::process(),
                candidate: &execution,
            },
            &ryo_benchmark::outbound::inventory::factory::local(),
            evidence.as_ref(),
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

pub(super) fn reference(executable: PathBuf, tool: Tool) -> Result<LinterRuffAdapter, Error> {
    let target_version = ruff_target(configured(&tool.parser_configuration))?
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
/// The projection of the parser's diagnostics to the `invalid-syntax` rule, as every run of the candidate in this composition makes it.
///
/// # Errors
///
/// A rule id or a query the SDK refuses.
pub(super) fn frontend_diagnostic()
-> Result<ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection, Error> {
    Ok(
        ryo_benchmark::outbound::engine::types::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from(constants::INVALID_SYNTAX)
                .map_err(|error| Error::Invalid(error.to_string()))?,
            query: ryo::query::RyoQlQuery::try_from(
                crate::frontend_projection::constants::FRONTEND_DIAGNOSTIC_QUERY,
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        },
    )
}

/// The target configuration a parser selection names, none when it names none.
pub(super) fn configured(
    selection: &ryo_benchmark_contracts::domain::requests::types::ParserSelection,
) -> Option<TargetConfiguration<'_>> {
    match selection {
        ryo_benchmark_contracts::domain::requests::types::ParserSelection::Named(name) => {
            Some(TargetConfiguration(name.0.as_str()))
        }
        ryo_benchmark_contracts::domain::requests::types::ParserSelection::Unspecified => None,
    }
}

/// The linked parser's target version for a target configuration, the same identity the reference is given.
pub(super) fn parser_target(
    configuration: Option<TargetConfiguration<'_>>,
) -> Result<ryo_python::settings::PythonTargetVersion, Error> {
    Ok(super::targets::resolve(configuration)?.parser)
}

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
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

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
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

// Composition is not DRY: this port wiring repeats in each linter's composition by the owner's rule; only pure values are shared.
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
