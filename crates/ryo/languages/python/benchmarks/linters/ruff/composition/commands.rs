//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo_benchmark::domain::suite::types::CaseCount;
use ryo_benchmark::domain::suite::types::Tool;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use std::{path::PathBuf, sync::Arc};

pub(super) fn worker(
    mut args: impl Iterator<Item = BenchmarkArgument>,
    tracing: WorkerTracing,
) -> Result<(), Error> {
    let input = args
        .next()
        .map(|argument| argument.0)
        .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked()]);
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort> =
        match if tracing == WorkerTracing::Enabled {
            Some(
                args.next()
                    .map(|argument| argument.0)
                    .ok_or_else(|| Error::Invalid(constants::MISSING_TRACE.into()))?,
            )
        } else {
            None
        } {
            Some(path) => Arc::new(ryo_benchmark::outbound::engine::factory::local(
                languages,
                Some(path.into()),
            )),
            None => Arc::new(ryo_benchmark::outbound::engine::factory::local(
                languages, None,
            )),
        };
    let transport =
        ryo_benchmark::outbound::worker::factory::json(input.into(), Box::new(std::io::stdout()));
    RunRyoUseCaseImpl::new(engine, Arc::new(transport)).run_ryo()
}

pub(super) fn verify_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<CaseCount, Error> {
    let suite_source = Arc::new(ryo_benchmark::outbound::suite::factory::local());
    let execution = Arc::new(ryo_benchmark::outbound::execution::factory::process());
    let executable = PathBuf::from(
        args.next()
            .map(|argument| argument.0)
            .ok_or_else(|| Error::Invalid(constants::EXPECTED_RUFF.into()))?,
    )
    .canonicalize()?;
    let mut tool = suite_source.profile(ArtifactPath(&directory()))?;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let audit = suite_source.audit(ArtifactPath(&directory()), &tool)?;
    ryo_benchmark::domain::suite::service::require_accounted(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let reference = Arc::new(reference(executable, tool));
    let scratch = tempfile::tempdir()?;
    let root = scratch.path().canonicalize()?;
    let profile = root.join(constants::PROFILE_DIRECTORY);
    suite_source.stage(ArtifactPath(&directory()), ArtifactPath(&profile), &audit)?;
    let candidate = Arc::new(ryo_benchmark::outbound::linter::factory::ryo(
        std::env::current_exe()?,
        profile.join(constants::RULES_DIRECTORY),
    ));
    let count = ryo_benchmark::application::run::service::verify_accounted(
        suite_source.as_ref(),
        reference.as_ref(),
        candidate.as_ref(),
        execution.as_ref(),
        &profile,
        &root.join(constants::CASES_DIRECTORY),
    )?;
    Ok(count)
}

pub(super) fn reference(executable: PathBuf, tool: Tool) -> LinterRuffAdapter {
    LinterRuffAdapter::new(LinterRuffSettings {
        executable,
        defaults: tool.defaults.into_iter().collect(),
    })
}

/// Text supplied as one executable command argument.
pub(super) struct BenchmarkArgument(pub String);
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkerTracing {
    Disabled,
    Enabled,
}
pub(super) fn directory() -> PathBuf {
    std::path::Path::new(constants::MANIFEST_DIRECTORY).join(constants::RUFF_DIRECTORY)
}

pub(super) fn collector(
    suite: &dyn SuitePort,
    source: &std::path::Path,
    workload: &std::path::Path,
) -> Result<CollectorIdentity, Error> {
    let executable = workload.join(constants::COLLECTOR_EXECUTABLE);
    let _bytes = std::fs::copy(source, &executable)?;
    let digest = suite.digest(ArtifactPath(&executable))?;
    std::fs::write(workload.join(constants::COLLECTOR_IDENTITY), &digest.0)?;
    Ok(CollectorIdentity { executable, digest })
}

pub(super) struct CollectorIdentity {
    pub(super) executable: PathBuf,
    pub(super) digest: ryo_benchmark::domain::suite::types::ArtifactDigest,
}

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
