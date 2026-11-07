//! Verification-only raw capture; measured runs keep their existing allocation path.
use ryo_benchmark_contracts::domain::documents::types::{observation_document::ExitStatus, text::{CanonicalJson, PathText, ReasonText, Sha256Text, YesNo}};
use super::constants;
use super::types::{AttemptOutcome, CaptureRecord, ProcessCapture};
use crate::domain::diagnostics::service::validate_coverage;
use crate::domain::rule_context::service::admit_configuration;
use crate::outbound::{
    context::interface::ContextPort,
    execution::interface::ExecutionPort,
    execution::types::ExitObservationMode,
    inventory::types::{
        ArgumentText, CompleteInvocation, EnvironmentName, EnvironmentValue, FailedInvocation,
        InvocationDocument, NormalizedSettings, Pin, Signal,
    },
    linter::interface::LinterPort,
    suite::{
        interface::SuitePort,
        types::ArtifactPath,
    },
};
use ryo_benchmark_contracts::domain::artifacts::types::FileMetadataEntry;
use ryo_benchmark_contracts::domain::measurements::types::{
    Invocation, Measurement, ProcessOutput, TerminationSignal,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{ContextSeal, RunRequest};
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::collections::BTreeMap;
use std::path::Path;

/// One verification preparation boundary for both contestants and every attempt.
fn prepare(
    suite: &dyn SuitePort,
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    request: &RunRequest,
) -> Result<Invocation, Error> {
    crate::domain::suite::service::validate_selected_rules(&request.selected_rules)?;
    admit_configuration(request, linter.rule_configuration())?;
    suite.validate_file_metadata(request)?;
    if let ContextSeal::Sealed(manifest) = &request.context_manifest {
        context.validate(manifest, &request.root)?;
    }
    linter.prepare(request)
}

/// Decode a captured output into a measurement, refusing a report that does not cover the request.
fn decoded(
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    request: &RunRequest,
    output: ProcessOutput,
) -> Result<Measurement, Error> {
    let wall = output.wall;
    let resources = output.resources;
    let report = linter.decode(request, output)?;
    if let (ContextSeal::Sealed(manifest), ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt)) =
        (&request.context_manifest, &report.project_context)
    {
        context.verify_receipt(manifest, &request.root, receipt)?;
    }
    validate_coverage(&request.files, &report)?;
    Ok(Measurement {
        wall,
        resources,
        report,
    })
}

/// The candidate keeps ordinary report decoding while sharing reference preparation admission.
///
/// # Errors
/// Propagates preparation, launch, decoding and coverage failures.
pub fn candidate(
    suite: &dyn SuitePort,
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    let invocation = prepare(suite, context, linter, request)?;
    decoded(context, linter, request, execution.run(invocation)?)
}

fn signal(output: &ProcessOutput) -> Signal {
    match output.signal {
        TerminationSignal::Exited => Signal::Absent,
        TerminationSignal::Signalled(number) => {
            Signal::Number(ExitStatus(i64::from(number.0)))
        }
    }
}

/// Execute once and preserve failed process streams before adapter errors discard them.
pub fn run(
    suite: &dyn SuitePort,
    context: &dyn ContextPort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    case: &PreparedCase,
    rules: &Path,
) -> AttemptOutcome {
    let spelled = |value: &std::ffi::OsStr| value.to_string_lossy().into_owned();
    let texts = |values: &[std::ffi::OsString]| {
        values
            .iter()
            .map(|value| ArgumentText(spelled(value)))
            .collect::<Vec<_>>()
    };
    let environment = |invocation: &Invocation| {
        invocation
            .environment
            .iter()
            .map(|(key, value)| (EnvironmentName(spelled(key)), EnvironmentValue(spelled(value))))
            .collect::<BTreeMap<_, _>>()
    };
    let attempt = || {
        let invocation = prepare(suite, context, linter, &case.request)?;
        let output = execution.run(invocation.clone())?;
        Ok::<_, Error>((invocation, output))
    };
    let (invocation, output) = match attempt() {
        Ok(value) => value,
        Err(error) => {
            return AttemptOutcome {
                result: Err(error),
                capture: CaptureRecord::Absent,
            };
        }
    };
    // Verification artifacts are outside the measured path; preserve raw bytes only on decode failure.
    let raw = output.clone();
    let wall = output.wall;
    let resources = output.resources;
    match linter.decode(&case.request, output) {
        Ok(report) => AttemptOutcome {
            result: validate_report(context, &case.request, &report).map(|()| Measurement {
                wall,
                resources,
                report,
            }),
            capture: CaptureRecord::Absent,
        },
        Err(error) => {
            let observer = YesNo::from(matches!(
                execution.exit_observation(),
                ExitObservationMode::Direct
            ));
            let files = || {
                case.request
                    .files
                    .iter()
                    .map(|file| PathText(file.0.as_ref().to_owned()))
                    .collect::<Vec<_>>()
            };
            let metadata = || {
                let request_root =
                    case.request.root.to_str().ok_or_else(|| {
                        Error::Invalid(constants::PROCESS_ROOT_IS_NOT_UTF8.into())
                    })?;
                let state =
                    case.request.state.to_str().ok_or_else(|| {
                        Error::Invalid(constants::PROCESS_STATE_IS_NOT_UTF8.into())
                    })?;
                let profile_root = rules
                    .parent()
                    .ok_or_else(|| Error::Invalid(constants::MISSING_PROFILE_BOUNDARY.into()))?;
                let profile = suite.profile(ArtifactPath(profile_root))?;
                let executable = suite.digest(ArtifactPath(&invocation.executable))?;
                let normalized =
                    crate::domain::invocation_identity::service::normalize(&invocation, case)
                        .map_err(|error| Error::Invalid(error.to_string()))?;
                let settings_text = linter.settings_text(&normalized, case, &profile)?.0;
                let settings = crate::domain::canonical_text::service::text_digest(
                    &crate::domain::canonical_text::types::Document(settings_text.clone()),
                );
                let profile_pin = suite.digest(ArtifactPath(
                    &profile_root.join(crate::domain::suite::constants::BENCHMARK_TOML),
                ))?;
                let metadata_pin =
                    crate::domain::input_metadata::service::digest(&case.request.file_metadata)?;
                Ok::<_, Error>(InvocationDocument::Complete(CompleteInvocation {
                    actual_request_root: PathText(request_root.to_owned()),
                    files: files(),
                    executable_sha256: Sha256Text(executable.0),
                    settings_sha256: Sha256Text(settings.0),
                    normalized_settings: NormalizedSettings {
                        arguments: texts(&normalized.arguments),
                        environment: environment(&normalized),
                        directory: PathText(spelled(normalized.directory.as_os_str())),
                        canonical: CanonicalJson(settings_text),
                    },
                    profile_sha256: Sha256Text(profile_pin.0),
                    tool: ReasonText(profile.tool.0),
                    version: ReasonText(profile.version.0),
                    state: PathText(state.to_owned()),
                    arguments: texts(&invocation.arguments),
                    environment: environment(&invocation),
                    directory: PathText(spelled(invocation.directory.as_os_str())),
                    exit: ExitStatus(i64::from(raw.status.0)),
                    signal: signal(&raw),
                    observer_no_deadline: observer,
                    source_sha256: Pin::Absent,
                    file_metadata_sha256: metadata_pin
                        .map_or(Pin::Absent, |digest| Pin::Sha256(Sha256Text(digest.0))),
                }))
            };
            match metadata() {
                Ok(invocation) => AttemptOutcome {
                    result: Err(error),
                    capture: CaptureRecord::Kept(ProcessCapture {
                        invocation,
                        output: raw,
                    }),
                },
                Err(capture_error) => {
                    let invocation = InvocationDocument::Failed(FailedInvocation {
                        actual_request_root: PathText(
                            case.request.root.to_string_lossy().into_owned(),
                        ),
                        files: files(),
                        arguments: texts(&invocation.arguments),
                        environment: environment(&invocation),
                        directory: PathText(spelled(invocation.directory.as_os_str())),
                        exit: ExitStatus(i64::from(raw.status.0)),
                        signal: signal(&raw),
                        observer_no_deadline: observer,
                        metadata_error: ReasonText(capture_error.to_string()),
                        source_sha256: Pin::Absent,
                        file_metadata_sha256: Pin::Absent,
                    });
                    AttemptOutcome {
                        result: Err(Error::Invalid(format!(
                            "{error}; raw process identity unavailable: {capture_error}"
                        ))),
                        capture: CaptureRecord::Kept(ProcessCapture {
                            invocation,
                            output: raw,
                        }),
                    }
                }
            }
        }
    }
}

/// Validate decoded coverage after the context still holds, without treating a rejected report as a process failure.
fn validate_report(
    context: &dyn ContextPort,
    request: &RunRequest,
    report: &ryo_benchmark_contracts::domain::reports::types::RunReport,
) -> Result<(), Error> {
    if let (ContextSeal::Sealed(manifest), ryo_benchmark_contracts::domain::reports::types::ProjectContext::Observed(receipt)) =
        (&request.context_manifest, &report.project_context)
    {
        context.verify_receipt(manifest, &request.root, receipt)?;
    }
    validate_coverage(&request.files, report)
}
