//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use ryo_benchmark::domain::diagnostics::errors::DiagnosticsError as Error;
use ryo_benchmark::domain::suite::types::CaseCount;
use ryo_benchmark::domain::suite::types::Tool;
use ryo_benchmark::inbound::use_cases::run_ryo::{
    interface::RunRyoUseCase as _, use_case::RunRyoUseCaseImpl,
};
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use std::{path::PathBuf, sync::Arc};

/// Execute one explicit child request, optionally writing a separately labelled trace.
pub(super) fn worker(
    mut args: impl Iterator<Item = BenchmarkArgument>,
    tracing: WorkerTracing,
) -> Result<(), Error> {
    let input = args
        .next()
        .map(|argument| argument.0)
        .ok_or_else(|| Error::Invalid(constants::MISSING_WORKER_REQUEST.into()))?;
    let next_argument = args.next().map(|argument| argument.0);
    let (parser_configuration, trace_path) = match next_argument {
        Some(configuration) if configuration.starts_with("python-target=") => {
            let trace_path = if tracing == WorkerTracing::Enabled {
                Some(
                    args.next()
                        .map(|argument| argument.0)
                        .ok_or_else(|| Error::Invalid(constants::MISSING_TRACE.into()))?,
                )
            } else {
                None
            };
            (Some(configuration), trace_path)
        }
        Some(path) if tracing == WorkerTracing::Enabled => (None, Some(path)),
        Some(_) => return Err(Error::Invalid("unexpected worker configuration".into())),
        None if tracing == WorkerTracing::Enabled => {
            return Err(Error::Invalid(constants::MISSING_TRACE.into()));
        }
        None => (None, None),
    };
    if let Some(configuration) = parser_configuration.as_deref() {
        let request: serde_json::Value =
            serde_json::from_str(&input).map_err(|error| Error::Invalid(error.to_string()))?;
        if request
            .get("parser_configuration")
            .and_then(serde_json::Value::as_str)
            != Some(configuration)
        {
            return Err(Error::Invalid(
                "worker parser configuration differs from its profile identity".into(),
            ));
        }
    }
    let target_version = parser_target(parser_configuration.as_deref())?;
    let languages = ryo::linter::Languages(vec![ryo_python::linked_with_settings(
        ryo_python::settings::PythonParserSettings::new(target_version),
    )]);
    let frontend_diagnostic =
        ryo_benchmark::outbound::engine::factory::FrontendDiagnosticProjection {
            rule: ryo::linter::RuleId::try_from(constants::INVALID_SYNTAX)
                .map_err(|error| Error::Invalid(error.to_string()))?,
            query: ryo::query::RyoQlQuery::try_from(crate::constants::FRONTEND_DIAGNOSTIC_QUERY)
                .map_err(|error| Error::Invalid(error.to_string()))?,
        };
    let engine: Arc<dyn ryo_benchmark::outbound::engine::interface::EnginePort> = match trace_path {
        Some(path) => Arc::new(
            ryo_benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                languages,
                Some(path.into()),
                frontend_diagnostic.clone(),
            ),
        ),
        None => Arc::new(
            ryo_benchmark::outbound::engine::factory::local_with_frontend_diagnostic(
                languages,
                None,
                frontend_diagnostic,
            ),
        ),
    };
    let transport =
        ryo_benchmark::outbound::worker::factory::json(input.into(), Box::new(std::io::stdout()));
    RunRyoUseCaseImpl::new(engine, Arc::new(transport)).run_ryo()
}

/// Verify pinned tools, complete rule accounting and independently authored findings.
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
    let source = args
        .next()
        .map_or_else(directory, |argument| PathBuf::from(argument.0));
    let mut tool = suite_source.profile(ArtifactPath(&source))?;
    let query_modules = tool.query_modules.clone();
    let parser_configuration = tool.parser_configuration.clone();
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let audit = suite_source.audit(ArtifactPath(&source), &tool)?;
    ryo_benchmark::domain::suite::service::require_accounted(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let reference = Arc::new(reference(executable, tool)?);
    let scratch = tempfile::tempdir()?;
    let root = scratch.path().canonicalize()?;
    let profile = root.join(constants::PROFILE_DIRECTORY);
    suite_source.stage(ArtifactPath(&source), ArtifactPath(&profile), &audit)?;
    let candidate = Arc::new(ryo_benchmark::outbound::linter::factory::ryo(
        std::env::current_exe()?,
        profile.join(constants::RULES_DIRECTORY),
        query_modules,
        parser_configuration,
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

/// Validate authored fixture expectations against Ruff without loading a Ryo candidate.
pub(super) fn verify_reference_cases(
    mut args: impl Iterator<Item = BenchmarkArgument>,
) -> Result<ryo_benchmark::domain::suite::types::ReferenceCaseCount, Error> {
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
    let scratch = tempfile::tempdir()?;
    ryo_benchmark::application::run::service::verify_reference_cases(
        &suite_source,
        &reference,
        &execution,
        &source.join(constants::RULES_DIRECTORY),
        scratch.path(),
    )
}

/// Compose the selected reference rule subset with its verified executable.
pub(super) fn reference(executable: PathBuf, tool: Tool) -> Result<LinterRuffAdapter, Error> {
    let target_version = ruff_target(tool.parser_configuration.as_deref())?.to_owned();
    Ok(LinterRuffAdapter::new(LinterRuffSettings {
        executable,
        defaults: tool.defaults.into_iter().collect(),
        target_version,
    }))
}

/// Map the sealed shared identity to Ruff's explicit CLI value.
fn ruff_target(configuration: Option<&str>) -> Result<&'static str, Error> {
    match configuration {
        None | Some("python-target=3.10") => Ok("py310"),
        Some("python-target=3.11") => Ok("py311"),
        Some(_) => Err(Error::Invalid(
            "unsupported Ruff parser configuration".into(),
        )),
    }
}

/// Map the same sealed identity to the linked Python parser settings.
fn parser_target(
    configuration: Option<&str>,
) -> Result<ryo_python::settings::PythonTargetVersion, Error> {
    match configuration {
        None | Some("python-target=3.10") => Ok(ryo_python::settings::PythonTargetVersion::Py310),
        Some("python-target=3.11") => Ok(ryo_python::settings::PythonTargetVersion::Py311),
        Some(_) => Err(Error::Invalid(
            "unsupported Ruff parser configuration".into(),
        )),
    }
}

/// Text supplied as one executable command argument.
pub(super) struct BenchmarkArgument(pub String);
/// Whether this worker invocation records a separate profiling trace.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum WorkerTracing {
    /// Ordinary correctness or timing invocation.
    Disabled,
    /// Explicit profiling invocation excluded from timing observations.
    Enabled,
}

#[cfg(test)]
mod target_configuration_tests {
    use super::{parser_target, ruff_target};
    use ryo_python::settings::PythonTargetVersion;

    #[test]
    fn target_configuration_is_shared_by_reference_and_candidate() {
        for (identity, cli, parser) in [
            (None, "py310", PythonTargetVersion::Py310),
            (
                Some("python-target=3.11"),
                "py311",
                PythonTargetVersion::Py311,
            ),
        ] {
            assert_eq!(ruff_target(identity).expect("supported target"), cli);
            assert_eq!(parser_target(identity).expect("supported target"), parser);
        }
    }

    #[test]
    fn unsupported_target_configuration_is_refused() {
        assert!(ruff_target(Some("python-target=3.12")).is_err());
        assert!(parser_target(Some("python-target=3.12")).is_err());
    }
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
    let _bytes = std::fs::copy(source, &executable)?;
    let digest = suite.digest(ArtifactPath(&executable))?;
    std::fs::write(workload.join(constants::COLLECTOR_IDENTITY), &digest.0)?;
    Ok(CollectorIdentity { executable, digest })
}

/// Copied collector identity; certification subsequently seals these artifacts.
pub(super) struct CollectorIdentity {
    /// Frozen native collector path.
    pub executable: PathBuf,
    /// Digest computed from the copied bytes.
    pub digest: ryo_benchmark::domain::suite::types::ArtifactDigest,
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
    suite_source: &dyn SuitePort,
    output: &mut dyn std::io::Write,
) -> Result<(), Error> {
    let tool = suite_source.profile(ArtifactPath(&directory()))?;
    let result = suite_source.audit(ArtifactPath(&directory()), &tool)?;
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
