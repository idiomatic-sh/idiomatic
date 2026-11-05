//! Verification-only raw capture; measured runs keep their existing allocation path.
use crate::outbound::{
    execution::interface::ExecutionPort,
    linter::interface::LinterPort,
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::artifacts::types::{FileExecutable, FileMetadataEntry};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::PreparedCase;
use std::path::Path;

/// Raw process bytes exist independently of whether a report could be decoded.
pub(super) struct Capture {
    pub invocation: serde_json::Value,
    pub output: ProcessOutput,
}
/// One actual attempt, including operational failures without a fabricated report.
pub(super) struct Outcome {
    pub result: Result<Measurement, Error>,
    pub capture: Option<Capture>,
}
/// One verification preparation boundary for both contestants and every attempt.
fn prepare(
    suite: &dyn SuitePort,
    linter: &dyn LinterPort,
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
) -> Result<ryo_benchmark_contracts::domain::measurements::types::Invocation, Error> {
    crate::domain::suite::service::validate_selected_rules(request.selected_rules.as_deref())?;
    configuration(linter, request)?;
    suite.validate_file_metadata(request)?;
    if let Some(context) = &request.context_manifest {
        crate::outbound::context::adapters::local::adapter::validate(context, &request.root)?;
    }
    linter.prepare(request)
}
/// The candidate keeps ordinary report decoding while sharing reference preparation admission.
pub(super) fn candidate(
    suite: &dyn SuitePort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
) -> Result<Measurement, Error> {
    let invocation = prepare(suite, linter, request)?;
    super::service::finish(linter, request, execution.run(invocation)?)
}
/// Execute once and preserve failed process streams before adapter errors discard them.
pub(super) fn run(
    suite: &dyn SuitePort,
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    case: &PreparedCase,
    rules: &Path,
) -> Outcome {
    let attempt = || {
        let invocation = prepare(suite, linter, &case.request)?;
        let output = execution.run(invocation.clone())?;
        Ok::<_, Error>((invocation, output))
    };
    let (invocation, output) = match attempt() {
        Ok(value) => value,
        Err(error) => {
            return Outcome {
                result: Err(error),
                capture: None,
            };
        }
    };
    // Verification artifacts are outside the measured path; preserve raw bytes only on decode failure.
    let raw = output.clone();
    let wall = output.wall;
    let resources = output.resources;
    match linter.decode(&case.request, output) {
        Ok(report) => Outcome {
            result: super::service::validate_report(&case.request, &report).map(|()| Measurement {
                wall,
                resources,
                report,
            }),
            capture: None,
        },
        Err(error) => {
            let metadata = || {
                let request_root = case
                    .request
                    .root
                    .to_str()
                    .ok_or_else(|| Error::Invalid("process root is not UTF8".into()))?;
                let state = case
                    .request
                    .state
                    .to_str()
                    .ok_or_else(|| Error::Invalid("process state is not UTF8".into()))?;
                let profile_root = rules
                    .parent()
                    .ok_or_else(|| Error::Invalid("missing profile boundary".into()))?;
                let profile = suite.profile(ArtifactPath(profile_root))?;
                let executable = suite.digest(ArtifactPath(&invocation.executable))?;
                let settings =
                    super::invocation_identity::invocation_settings(&invocation, case, &profile)?;
                let mut captured = serde_json::json!({
                    "actual_request_root":request_root, "files":case.request.files.iter().map(|file|file.0.as_ref()).collect::<Vec<_>>(),
                    "executable_sha256":executable.0, "settings_sha256":settings.0,
                    "normalized_settings":super::invocation_identity::normalized_invocation(&invocation,case,&profile)?,
                    "profile_sha256":suite.digest(ArtifactPath(&profile_root.join("benchmark.toml")))?.0,
                    "tool":profile.tool.0, "version":profile.version.0, "state":state,
                    "arguments":invocation.arguments.iter().map(|argument|argument.to_string_lossy()).collect::<Vec<_>>(),
                    "environment":invocation.environment.iter().map(|(key,value)|(key.to_string_lossy(),value.to_string_lossy())).collect::<std::collections::BTreeMap<_,_>>(),
                    "directory":invocation.directory.to_string_lossy(), "exit":raw.status.0, "signal":raw.signal,
                    "observer_no_deadline":execution.observes_completed_processes_without_deadline(),
                });
                if let Some(digest) = ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest::file_metadata((case.request.file_metadata.as_ref()).map(|values| values.iter().map(|(path, executable)| FileMetadataEntry { path, executable: if *executable { FileExecutable::Set } else { FileExecutable::Unset } })))? {
                    captured["file_metadata_sha256"] = serde_json::json!(digest.0);
                }
                Ok::<_, Error>(captured)
            };
            match metadata() {
                Ok(invocation) => Outcome {
                    result: Err(error),
                    capture: Some(Capture {
                        invocation,
                        output: raw,
                    }),
                },
                Err(capture_error) => {
                    let invocation = serde_json::json!({
                        "actual_request_root":case.request.root.to_string_lossy(),
                        "files":case.request.files.iter().map(|file|file.0.as_ref()).collect::<Vec<_>>(),
                        "arguments":invocation.arguments.iter().map(|argument|argument.to_string_lossy()).collect::<Vec<_>>(),
                        "environment":invocation.environment.iter().map(|(key,value)|(key.to_string_lossy(),value.to_string_lossy())).collect::<std::collections::BTreeMap<_,_>>(),
                        "directory":invocation.directory.to_string_lossy(), "exit":raw.status.0, "signal":raw.signal,
                        "observer_no_deadline":execution.observes_completed_processes_without_deadline(),
                        "metadata_error":capture_error.to_string(),
                    });
                    Outcome {
                        result: Err(Error::Invalid(format!(
                            "{error}; raw process identity unavailable: {capture_error}"
                        ))),
                        capture: Some(Capture {
                            invocation,
                            output: raw,
                        }),
                    }
                }
            }
        }
    }
}

#[cfg(all(test, unix))]
mod input_metadata_test;

/// One shared configuration admission boundary for both contestants and measured/verification calls.
pub(super) fn configuration(
    linter: &dyn LinterPort,
    request: &ryo_benchmark_contracts::domain::requests::types::RunRequest,
) -> Result<(), Error> {
    if let Some(context) = &request.rule_context {
        crate::domain::suite::rule_context::check(context, &request.files)?;
        crate::domain::suite::rule_context::check_selection(
            context,
            request.selected_rules.as_deref(),
        )?;
        if linter.rule_configuration()
            != ryo_benchmark_contracts::domain::workloads::types::RuleConfigurationSupport::Explicit
        {
            return Err(Error::Invalid(
                "contestant does not implement the declared rule configuration contract".into(),
            ));
        }
    }
    Ok(())
}
