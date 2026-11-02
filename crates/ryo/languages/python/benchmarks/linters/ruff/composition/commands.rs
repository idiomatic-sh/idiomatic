//! Independent worker and fixture commands at the Cargo executable boundary.
use super::super::{adapter::LinterRuffAdapter, settings::LinterRuffSettings};
use super::constants;
use super::diagnostic_protocol::WorkerTracing;
use super::request_input::{BenchmarkArgument, worker_input};
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
    let input = worker_input(&mut args)?;
    let next_argument = args.next().map(|argument| argument.0);
    let (parser_configuration, trace_path) = match next_argument {
        Some(configuration) if configuration.starts_with("python-target=") => {
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
        Some(_) => return Err(Error::Invalid("unexpected worker configuration".into())),
        None if tracing != WorkerTracing::Disabled => {
            return Err(Error::Invalid(constants::MISSING_TRACE.into()));
        }
        None => (None, None),
    };
    if args.next().is_some() {
        return Err(Error::Invalid("unexpected trailing worker argument".into()));
    }
    super::request_input::checked_worker_identity(&input, parser_configuration.as_deref())?;
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
    let mut shard = None;
    let mut case_selection = None;
    for argument in args {
        if argument.0.starts_with("--shard=") {
            if shard.replace(parse_shard(&argument.0)?).is_some() {
                return Err(Error::Invalid("duplicate case shard argument".into()));
            }
        } else if let Some(path) = argument.0.strip_prefix("--case-selection=") {
            if case_selection.is_some() {
                return Err(Error::Invalid("duplicate case selection argument".into()));
            }
            let selected: Vec<String> = serde_json::from_slice(&std::fs::read(path)?)
                .map_err(|error| Error::Invalid(error.to_string()))?;
            let selected_count = selected.len();
            let selected = selected
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>();
            if selected.is_empty() || selected.len() != selected_count {
                return Err(Error::Invalid(
                    "case selection must be nonempty and unique".into(),
                ));
            }
            case_selection = Some(selected);
        } else {
            return Err(Error::Invalid("unexpected verify-cases argument".into()));
        }
    }
    let mut tool = suite_source.profile(ArtifactPath(&source))?;
    let query_modules = tool.query_modules.clone();
    let rule_applicability = tool.rule_applicability.clone();
    let parser_configuration = tool.parser_configuration.clone();
    let source_language = tool.source_language.clone();
    let project_context_mode = tool.project_context_mode;
    suite_source.verify_tool(ArtifactPath(&executable), &tool)?;
    let audit = suite_source.audit(ArtifactPath(&source), &tool)?;
    ryo_benchmark::domain::suite::service::require_accounted(&audit, &tool)?;
    tool.defaults = audit.authored().to_vec();
    let reference = Arc::new(reference(executable, tool)?);
    let scratch = tempfile::tempdir()?;
    let root = scratch.path().canonicalize()?;
    let profile = root.join(constants::PROFILE_DIRECTORY);
    suite_source.stage(ArtifactPath(&source), ArtifactPath(&profile), &audit)?;
    let candidate = Arc::new(ryo_benchmark::outbound::linter::factory::ryo_context(
        std::env::current_exe()?,
        profile.join(constants::RULES_DIRECTORY),
        query_modules,
        rule_applicability,
        parser_configuration,
        source_language,
        project_context_mode,
    ));
    let count = ryo_benchmark::application::run::service::verify_accounted_shard(
        suite_source.as_ref(),
        reference.as_ref(),
        candidate.as_ref(),
        execution.as_ref(),
        &profile,
        &root.join(constants::CASES_DIRECTORY),
        ryo_benchmark::application::run::service::CaseSelection {
            shard,
            ids: case_selection.as_ref(),
        },
    )?;
    Ok(count)
}

/// Parse explicit zero-based shard coordinates using the validated application type.
fn parse_shard(value: &str) -> Result<ryo_benchmark::application::run::service::CaseShard, Error> {
    let coordinates = value
        .strip_prefix("--shard=")
        .ok_or_else(|| Error::Invalid("expected --shard=INDEX/COUNT".into()))?;
    let (index, count) = coordinates
        .split_once('/')
        .ok_or_else(|| Error::Invalid("expected --shard=INDEX/COUNT".into()))?;
    let index = index
        .parse()
        .map_err(|_invalid_index| Error::Invalid("invalid case shard index".into()))?;
    let count = count
        .parse()
        .map_err(|_invalid_count| Error::Invalid("invalid case shard count".into()))?;
    ryo_benchmark::application::run::service::CaseShard::new(index, count)
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
    Ok(super::targets::resolve(configuration)?.reference)
}

/// Use exactly the same explicit identity for the linked parser as the reference.
fn parser_target(
    configuration: Option<&str>,
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
