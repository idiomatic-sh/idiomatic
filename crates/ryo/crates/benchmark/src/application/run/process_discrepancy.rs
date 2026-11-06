//! Convert explicit process wire contracts; actual matching remains shared and byte-exact.
use super::discrepancy::digest;
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
            crate::application::run::constants::PROCESS_FAILURE_REQUIRES_A_POSITIVE_NUMERIC.into(),
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
        OutputPart::Literal(value) => serde_json::json!({crate::application::run::constants::KIND:crate::application::run::constants::LITERAL, crate::application::run::constants::VALUE:value}),
        OutputPart::InputPath { file, encoding } => serde_json::json!({crate::application::run::constants::KIND:crate::application::run::constants::INPUT_PATH, crate::application::run::constants::FILE:file, crate::application::run::constants::ENCODING:match encoding { PathEncoding::Utf8Bytes => crate::application::run::constants::UTF8_BYTES, PathEncoding::JsonStringContent => crate::application::run::constants::JSON_STRING_CONTENT }}),
    }).collect::<Vec<_>>()
    };
    super::discrepancy::json_digest(
        &serde_json::json!({crate::application::run::constants::KIND:crate::application::run::constants::PROCESS_FAILURE, crate::application::run::constants::EXIT:process.exit, crate::application::run::constants::SIGNAL:null, crate::application::run::constants::STDOUT:encode(&process.stdout), crate::application::run::constants::STDERR:encode(&process.stderr)}),
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
    let process = expected(case).ok_or_else(|| Error::Invalid(crate::application::run::constants::NO_PROCESS_EXPECTATION.into()))?;
    if validation[crate::application::run::constants::REFERENCE_EXPECTATION_KIND] != crate::application::run::constants::PROCESS_FAILURE
        || capture.invocation[crate::application::run::constants::EXECUTABLE_SHA256] != validation[crate::application::run::constants::ACTUAL_EXECUTABLE_SHA256]
        || capture.invocation[crate::application::run::constants::SETTINGS_SHA256] != validation[crate::application::run::constants::SETTINGS_SHA256]
        || capture.invocation[crate::application::run::constants::PROFILE_SHA256] != validation[crate::application::run::constants::PROFILE_SHA256]
        || capture.invocation[crate::application::run::constants::FILE_METADATA_SHA256] != validation[crate::application::run::constants::FILE_METADATA_SHA256]
    {
        return Err(Error::Invalid(
            crate::application::run::constants::ACTUAL_PROCESS_IDENTITY_IS_NOT_THE.into(),
        ));
    }

    if capture.output.signal.is_some()
        || capture.output.status.0 != process.exit
        || capture.invocation[crate::application::run::constants::OBSERVER_NO_DEADLINE] != true
    {
        return Err(Error::Invalid(
            crate::application::run::constants::REFERENCE_TERMINATION_DIFFERS_FROM_DECLARED.into(),
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
            crate::application::run::constants::REFERENCE_PROCESS_BYTES_DIFFER_FROM_DECLARED.into(),
        ));
    }
    let profile_root = rules
        .parent()
        .ok_or_else(|| Error::Invalid(crate::application::run::constants::MISSING_PROCESS_PROFILE_BOUNDARY.into()))?;
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
            file_metadata_sha256: validation[crate::application::run::constants::FILE_METADATA_SHA256].as_str(),
            source_sha256: validation[crate::application::run::constants::SOURCE_SHA256]
                .as_str()
                .ok_or_else(|| Error::Invalid(crate::application::run::constants::MISSING_VALIDATED_SOURCE_IDENTITY.into()))?,
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
        expected(case).ok_or_else(|| Error::Invalid(crate::application::run::constants::MISSING_PROCESS_CONTRACT.into()))?;
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
                crate::application::run::constants::PROCESS_EVIDENCE_HAS_AN_UNAPPROVED_EXACT.into(),
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
                    crate::application::run::constants::PROCESS_OBSERVATIONS_MUST_RETAIN_SEPARATE_RA.into(),
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
            crate::application::run::constants::PROCESS_EXPECTATION_NEEDS_TWO_CAPTURES_FOR.into(),
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
                .ok_or_else(|| Error::Invalid(crate::application::run::constants::MISSING_RAW_INVOCATION.into()))?,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let root = metadata[crate::application::run::constants::ACTUAL_REQUEST_ROOT]
            .as_str()
            .ok_or_else(|| Error::Invalid(crate::application::run::constants::ARCHIVED_REQUEST_ROOT_IS_MISSING.into()))?;
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
        if !metadata[crate::application::run::constants::METADATA_ERROR].is_null()
            || metadata[crate::application::run::constants::FILES] != serde_json::json!(required_files)
            || metadata[crate::application::run::constants::EXECUTABLE_SHA256] != observation.executable_sha256.0
            || metadata[crate::application::run::constants::PROFILE_SHA256] != identity.profile_sha256.0
            || metadata[crate::application::run::constants::SOURCE_SHA256] != identity.source_sha256.0
            || metadata[crate::application::run::constants::FILE_METADATA_SHA256]
                != serde_json::json!(identity.file_metadata_sha256.as_ref().map(|value| &value.0))
            || metadata[crate::application::run::constants::SETTINGS_SHA256] != identity.settings_sha256.0
            || metadata[crate::application::run::constants::TOOL] != profile.tool.0.as_str()
            || metadata[crate::application::run::constants::VERSION] != profile.version.0.as_str()
            || metadata[crate::application::run::constants::EXIT].as_i64() != Some(i64::from(process.exit))
            || metadata.get(crate::application::run::constants::SIGNAL) != Some(&serde_json::Value::Null)
            || metadata[crate::application::run::constants::OBSERVER_NO_DEADLINE] != true
            || bytes.get(&observation.stdout.path) != Some(&stdout)
            || bytes.get(&observation.stderr.path) != Some(&stderr)
        {
            return Err(Error::Invalid(
                crate::application::run::constants::ARCHIVED_PROCESS_BYTES_TERMINATION_OR_IMMUTA.into(),
            ));
        }
        let mut archived_case = case.clone();
        archived_case.request.root = root.into();
        archived_case.request.state = metadata[crate::application::run::constants::STATE]
            .as_str()
            .ok_or_else(|| Error::Invalid(crate::application::run::constants::ARCHIVED_STATE_BOUNDARY_IS_MISSING.into()))?
            .into();
        let arguments = serde_json::from_value::<Vec<String>>(metadata[crate::application::run::constants::ARGUMENTS].clone())
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let environment = serde_json::from_value::<std::collections::BTreeMap<String, String>>(
            metadata[crate::application::run::constants::ENVIRONMENT].clone(),
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let invocation = ryo_benchmark_contracts::domain::measurements::types::Invocation {
            executable: std::path::PathBuf::new(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            environment: environment
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            directory: metadata[crate::application::run::constants::DIRECTORY]
                .as_str()
                .ok_or_else(|| Error::Invalid(crate::application::run::constants::ARCHIVED_INVOCATION_DIRECTORY_IS_MISSING.into()))?
                .into(),
        };
        if super::invocation_identity::normalized_invocation(&invocation, &archived_case, profile)?
            != metadata[crate::application::run::constants::NORMALIZED_SETTINGS]
            || super::invocation_identity::invocation_settings(
                &invocation,
                &archived_case,
                profile,
            )? != identity.settings_sha256
        {
            return Err(Error::Invalid(
                crate::application::run::constants::ARCHIVED_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH.into(),
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
                    crate::application::run::constants::PROCESS_OBSERVATION_GROUPS_MUST_RETAIN_DISTI.into(),
                ));
            }
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
    }
    if groups.values().any(|count| *count < 2) || groups.get(executable).copied().unwrap_or(0) < 2 {
        return Err(Error::Invalid(
            crate::application::run::constants::MISSING_REPRODUCIBLE_EXACT_BINARY_OBSERVATIO.into(),
        ));
    }
    let mut rows = process
        .observations
        .iter()
        .filter(|row| row.executable_sha256.0 == executable)
        .map(|row| {
            serde_json::json!({
                crate::application::run::constants::EXECUTABLE_SHA256:row.executable_sha256.0,
                crate::application::run::constants::INVOCATION:{crate::application::run::constants::PATH:row.invocation.path,crate::application::run::constants::SHA256:row.invocation.sha256.0},
                crate::application::run::constants::STDOUT:{crate::application::run::constants::PATH:row.stdout.path,crate::application::run::constants::SHA256:row.stdout.sha256.0},
                crate::application::run::constants::STDERR:{crate::application::run::constants::PATH:row.stderr.path,crate::application::run::constants::SHA256:row.stderr.sha256.0},
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(serde_json::Value::to_string);
    Ok(super::discrepancy::json_digest(&serde_json::json!(rows)))
}
