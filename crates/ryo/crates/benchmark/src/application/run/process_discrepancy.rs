//! Convert explicit process wire contracts; actual matching remains shared and byte-exact.
use super::discrepancy::digest;
use crate::application::run::constants;
use crate::outbound::suite::types::process::{
    OutputPartWire, PathEncodingWire, ProcessArtifactWire, ProcessFailureWire,
};
use ryo_benchmark_contracts::domain::process::types::OutputPart;
use ryo_benchmark_contracts::domain::process::types::PathEncoding;
use ryo_benchmark_contracts::domain::process::types::ProcessArtifact;
use ryo_benchmark_contracts::domain::process::types::ProcessFailure;
use ryo_benchmark_contracts::domain::process::types::ProcessObservation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

fn artifact(value: ProcessArtifactWire) -> Result<ProcessArtifact, Error> {
    ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(&value.path))?;
    Ok(ProcessArtifact {
        path: value.path,
        sha256: digest(value.sha256)?,
    })
}
fn parts(values: Vec<OutputPartWire>) -> Result<Vec<OutputPart>, Error> {
    values
        .into_iter()
        .map(|value| match value {
            OutputPartWire::Literal { value } => Ok(OutputPart::Literal(value)),
            OutputPartWire::InputPath { file, encoding } => {
                ryo_benchmark_contracts::domain::artifacts::types::relative_artifact_path::value::RelativeArtifactPath::try_from(std::path::Path::new(&file))?;
                Ok(OutputPart::InputPath {
                    file,
                    encoding: match encoding {
                        PathEncodingWire::Utf8Bytes => PathEncoding::Utf8Bytes,
                        PathEncodingWire::JsonStringContent => PathEncoding::JsonStringContent,
                    },
                })
            }
        })
        .collect()
}
/// Decode only explicit tagged process expectations and preserve every raw artifact identity.
pub(super) fn convert(value: ProcessFailureWire) -> Result<ProcessFailure, Error> {
    let ProcessFailureWire::ProcessFailure {
        exit,
        stdout,
        stderr,
        observations,
    } = value;
    if exit <= 0 {
        return Err(Error::Invalid(
            constants::PROCESS_FAILURE_REQUIRES_A_POSITIVE_NUMERIC.into(),
        ));
    }
    Ok(ProcessFailure {
        exit,
        stdout: parts(stdout)?,
        stderr: parts(stderr)?,
        observations: observations
            .into_iter()
            .map(|value| {
                Ok(ProcessObservation {
                    executable_sha256: digest(value.executable_sha256)?,
                    invocation: artifact(value.invocation)?,
                    stdout: artifact(value.stdout)?,
                    stderr: artifact(value.stderr)?,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

/// Hash the exact template contract, without temporary physical roots.
pub(super) fn output_digest(
    process: &ProcessFailure,
) -> ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest {
    let encode = |parts: &[OutputPart]| {
        parts.iter().map(|part| match part {
        OutputPart::Literal(value) => serde_json::json!({constants::KIND:constants::LITERAL, constants::VALUE:value}),
        OutputPart::InputPath { file, encoding } => serde_json::json!({constants::KIND:constants::INPUT_PATH, constants::FILE:file, constants::ENCODING:match encoding { PathEncoding::Utf8Bytes => constants::UTF8_BYTES, PathEncoding::JsonStringContent => constants::JSON_STRING_CONTENT }}),
    }).collect::<Vec<_>>()
    };
    super::discrepancy::json_digest(
        &serde_json::json!({constants::KIND:constants::PROCESS_FAILURE, constants::EXIT:process.exit, constants::SIGNAL:null, constants::STDOUT:encode(&process.stdout), constants::STDERR:encode(&process.stderr)}),
    )
}

/// Closed expectation lookup; a process failure is never a diagnostic report.
pub(super) fn expected(
    case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
) -> Option<&ProcessFailure> {
    match case
        .reference_discrepancy
        .as_ref()
        .map(|item| &item.expected)
    {
        Some(
            ryo_benchmark_contracts::domain::process::types::ReferenceExpectation::ProcessFailure(
                process,
            ),
        ) => Some(process),
        _ => None,
    }
}
/// Require a completed numeric failure and every declared byte; no adapter error is itself evidence.
pub(super) fn matches(
    case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
    capture: &super::case_execution::Capture,
    validation: &serde_json::Value,
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    rules: &std::path::Path,
) -> Result<(), Error> {
    let process =
        expected(case).ok_or_else(|| Error::Invalid(constants::NO_PROCESS_EXPECTATION.into()))?;
    if validation[constants::REFERENCE_EXPECTATION_KIND] != constants::PROCESS_FAILURE
        || capture.invocation[constants::EXECUTABLE_SHA256]
            != validation[constants::ACTUAL_EXECUTABLE_SHA256]
        || capture.invocation[constants::SETTINGS_SHA256] != validation[constants::SETTINGS_SHA256]
        || capture.invocation[constants::PROFILE_SHA256] != validation[constants::PROFILE_SHA256]
        || capture.invocation[constants::FILE_METADATA_SHA256]
            != validation[constants::FILE_METADATA_SHA256]
    {
        return Err(Error::Invalid(
            constants::ACTUAL_PROCESS_IDENTITY_IS_NOT_THE.into(),
        ));
    }

    if capture.output.signal.is_some()
        || capture.output.status.0 != process.exit
        || capture.invocation[constants::OBSERVER_NO_DEADLINE] != true
    {
        return Err(Error::Invalid(
            constants::REFERENCE_TERMINATION_DIFFERS_FROM_DECLARED.into(),
        ));
    }
    let files = case
        .request
        .files
        .iter()
        .map(|file| std::path::PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    let stdout =
        crate::domain::suite::process::service::expand(&process.stdout, &case.request.root, &files)
            .map_err(Error::Invalid)?;
    let stderr =
        crate::domain::suite::process::service::expand(&process.stderr, &case.request.root, &files)
            .map_err(Error::Invalid)?;
    if stdout != capture.output.stdout.0 || stderr != capture.output.stderr.0 {
        return Err(Error::Invalid(
            constants::REFERENCE_PROCESS_BYTES_DIFFER_FROM_DECLARED.into(),
        ));
    }
    let profile_root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(constants::MISSING_PROCESS_PROFILE_BOUNDARY.into()))?;
    let metadata_files = case
        .request
        .file_metadata
        .as_ref()
        .map(|values| {
            values
                .keys()
                .map(|file| std::path::PathBuf::from(file.0.as_ref()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    suite
        .verify_discrepancy_artifacts(crate::outbound::suite::types::discrepancy::CaseArtifacts {
            profile: profile_root,
            root: &case.request.root,
            files: &files,
            file_metadata_files: &metadata_files,
            file_metadata_sha256: validation[constants::FILE_METADATA_SHA256].as_str(),
            source_sha256: validation[constants::SOURCE_SHA256]
                .as_str()
                .ok_or_else(|| {
                    Error::Invalid(constants::MISSING_VALIDATED_SOURCE_IDENTITY.into())
                })?,
            evidence: &[],
        })
        .map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(())
}

/// Validate every archived observation and require an exact pair for the actual binary.
pub(super) fn validate_archives(
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    profile_root: &std::path::Path,
    profile: &ryo_benchmark_contracts::domain::workloads::types::Tool,
    case: &ryo_benchmark_contracts::domain::workloads::types::PreparedCase,
    identity: &ryo_benchmark_contracts::domain::workloads::types::ReferenceDiscrepancyIdentity,
    actual: &ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest,
) -> Result<(), Error> {
    use crate::outbound::suite::types::{ArtifactPath, discrepancy::EvidenceArtifact};
    let process =
        expected(case).ok_or_else(|| Error::Invalid(constants::MISSING_PROCESS_CONTRACT.into()))?;
    let mut artifacts = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut groups = std::collections::BTreeMap::<&str, usize>::new();
    for observation in &process.observations {
        if !profile
            .sha256
            .iter()
            .any(|pin| pin.0 == observation.executable_sha256.0)
        {
            return Err(Error::Invalid(
                constants::PROCESS_EVIDENCE_HAS_AN_UNAPPROVED_EXACT.into(),
            ));
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
        for artifact in [
            &observation.invocation,
            &observation.stdout,
            &observation.stderr,
        ] {
            if !seen.insert(artifact.path.clone()) {
                return Err(Error::Invalid(
                    constants::PROCESS_OBSERVATIONS_MUST_RETAIN_SEPARATE_RA.into(),
                ));
            }
            artifacts.push(EvidenceArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.0.clone(),
                requirement:
                    crate::outbound::suite::types::discrepancy::EvidenceRequirement::Language,
            });
        }
    }
    if groups.is_empty()
        || groups.values().any(|count| *count < 2)
        || groups.get(actual.0.as_str()).copied().unwrap_or(0) < 2
    {
        return Err(Error::Invalid(
            constants::PROCESS_EXPECTATION_NEEDS_TWO_CAPTURES_FOR.into(),
        ));
    }
    let bytes = suite
        .read_discrepancy_artifacts(ArtifactPath(profile_root), &artifacts)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let files = case
        .request
        .files
        .iter()
        .map(|file| std::path::PathBuf::from(file.0.as_ref()))
        .collect::<Vec<_>>();
    for observation in &process.observations {
        let metadata: serde_json::Value = serde_json::from_slice(
            bytes
                .get(&observation.invocation.path)
                .ok_or_else(|| Error::Invalid(constants::MISSING_RAW_INVOCATION.into()))?,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let root = metadata[constants::ACTUAL_REQUEST_ROOT]
            .as_str()
            .ok_or_else(|| Error::Invalid(constants::ARCHIVED_REQUEST_ROOT_IS_MISSING.into()))?;
        let stdout = crate::domain::suite::process::service::expand(
            &process.stdout,
            std::path::Path::new(root),
            &files,
        )
        .map_err(Error::Invalid)?;
        let stderr = crate::domain::suite::process::service::expand(
            &process.stderr,
            std::path::Path::new(root),
            &files,
        )
        .map_err(Error::Invalid)?;
        let required_files = case
            .request
            .files
            .iter()
            .map(|file| file.0.as_ref())
            .collect::<Vec<_>>();
        if !metadata[constants::METADATA_ERROR].is_null()
            || metadata[constants::FILES] != serde_json::json!(required_files)
            || metadata[constants::EXECUTABLE_SHA256] != observation.executable_sha256.0
            || metadata[constants::PROFILE_SHA256] != identity.profile_sha256.0
            || metadata[constants::SOURCE_SHA256] != identity.source_sha256.0
            || metadata[constants::FILE_METADATA_SHA256]
                != serde_json::json!(identity.file_metadata_sha256.as_ref().map(|value| &value.0))
            || metadata[constants::SETTINGS_SHA256] != identity.settings_sha256.0
            || metadata[constants::TOOL] != profile.tool.0.as_str()
            || metadata[constants::VERSION] != profile.version.0.as_str()
            || metadata[constants::EXIT].as_i64() != Some(i64::from(process.exit))
            || metadata.get(constants::SIGNAL) != Some(&serde_json::Value::Null)
            || metadata[constants::OBSERVER_NO_DEADLINE] != true
            || bytes.get(&observation.stdout.path) != Some(&stdout)
            || bytes.get(&observation.stderr.path) != Some(&stderr)
        {
            return Err(Error::Invalid(
                constants::ARCHIVED_PROCESS_BYTES_TERMINATION_OR_IMMUTA.into(),
            ));
        }
        let mut archived_case = case.clone();
        archived_case.request.root = root.into();
        archived_case.request.state = metadata[constants::STATE]
            .as_str()
            .ok_or_else(|| Error::Invalid(constants::ARCHIVED_STATE_BOUNDARY_IS_MISSING.into()))?
            .into();
        let arguments =
            serde_json::from_value::<Vec<String>>(metadata[constants::ARGUMENTS].clone())
                .map_err(|error| Error::Invalid(error.to_string()))?;
        let environment = serde_json::from_value::<std::collections::BTreeMap<String, String>>(
            metadata[constants::ENVIRONMENT].clone(),
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let invocation = ryo_benchmark_contracts::domain::measurements::types::Invocation {
            executable: std::path::PathBuf::new(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            environment: environment
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            directory: metadata[constants::DIRECTORY]
                .as_str()
                .ok_or_else(|| {
                    Error::Invalid(constants::ARCHIVED_INVOCATION_DIRECTORY_IS_MISSING.into())
                })?
                .into(),
        };
        if super::invocation_identity::normalized_invocation(&invocation, &archived_case, profile)?
            != metadata[constants::NORMALIZED_SETTINGS]
            || super::invocation_identity::invocation_settings(
                &invocation,
                &archived_case,
                profile,
            )? != identity.settings_sha256
        {
            return Err(Error::Invalid(
                constants::ARCHIVED_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH.into(),
            ));
        }
    }
    Ok(())
}

/// Bind the chosen exact-binary archive group into both runtime and offline receipts.
pub(super) fn group_digest(
    process: &ProcessFailure,
    executable: &str,
) -> Result<ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest, Error>
{
    let mut groups = std::collections::BTreeMap::<&str, usize>::new();
    let mut artifacts = std::collections::BTreeSet::new();
    for observation in &process.observations {
        for raw in [
            &observation.invocation,
            &observation.stdout,
            &observation.stderr,
        ] {
            if !artifacts.insert(&raw.path) {
                return Err(Error::Invalid(
                    constants::PROCESS_OBSERVATION_GROUPS_MUST_RETAIN_DISTI.into(),
                ));
            }
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
    }
    if groups.values().any(|count| *count < 2) || groups.get(executable).copied().unwrap_or(0) < 2 {
        return Err(Error::Invalid(
            constants::MISSING_REPRODUCIBLE_EXACT_BINARY_OBSERVATIO.into(),
        ));
    }
    let mut rows = process
        .observations
        .iter()
        .filter(|row| row.executable_sha256.0 == executable)
        .map(|row| {
            serde_json::json!({
                constants::EXECUTABLE_SHA256:row.executable_sha256.0,
                constants::INVOCATION:{constants::PATH:row.invocation.path,constants::SHA256:row.invocation.sha256.0},
                constants::STDOUT:{constants::PATH:row.stdout.path,constants::SHA256:row.stdout.sha256.0},
                constants::STDERR:{constants::PATH:row.stderr.path,constants::SHA256:row.stderr.sha256.0},
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(serde_json::Value::to_string);
    Ok(super::discrepancy::json_digest(&serde_json::json!(rows)))
}
