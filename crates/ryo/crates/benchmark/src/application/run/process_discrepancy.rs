//! Convert explicit process wire contracts; actual matching remains shared and byte-exact.
use super::discrepancy::digest;
use crate::domain::{
    diagnostics::errors::DiagnosticsError as Error,
    suite::process::types::{
        OutputPart, PathEncoding, ProcessArtifact, ProcessFailure, ProcessObservation,
    },
};
use crate::outbound::suite::types::process::{
    OutputPartWire, PathEncodingWire, ProcessArtifactWire, ProcessFailureWire,
};

fn artifact(value: ProcessArtifactWire) -> Result<ProcessArtifact, Error> {
    crate::domain::suite::service::relative_path(&value.path)?;
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
                crate::domain::suite::service::relative_path(&file)?;
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
            "process failure requires a positive numeric exit".into(),
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
) -> crate::domain::suite::types::ArtifactDigest {
    let encode = |parts: &[OutputPart]| {
        parts.iter().map(|part| match part {
        OutputPart::Literal(value) => serde_json::json!({"kind":"literal", "value":value}),
        OutputPart::InputPath { file, encoding } => serde_json::json!({"kind":"input-path", "file":file, "encoding":match encoding { PathEncoding::Utf8Bytes => "utf8-bytes", PathEncoding::JsonStringContent => "json-string-content" }}),
    }).collect::<Vec<_>>()
    };
    super::discrepancy::json_digest(
        &serde_json::json!({"kind":"process-failure", "exit":process.exit, "signal":null, "stdout":encode(&process.stdout), "stderr":encode(&process.stderr)}),
    )
}

/// Closed expectation lookup; a process failure is never a diagnostic report.
pub(super) fn expected(
    case: &crate::domain::suite::types::PreparedCase,
) -> Option<&ProcessFailure> {
    match case
        .reference_discrepancy
        .as_ref()
        .map(|item| &item.expected)
    {
        Some(crate::domain::suite::process::types::ReferenceExpectation::ProcessFailure(
            process,
        )) => Some(process),
        _ => None,
    }
}
/// Require a completed numeric failure and every declared byte; no adapter error is itself evidence.
pub(super) fn matches(
    case: &crate::domain::suite::types::PreparedCase,
    capture: &super::case_execution::Capture,
    validation: &serde_json::Value,
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    rules: &std::path::Path,
) -> Result<(), Error> {
    let process = expected(case).ok_or_else(|| Error::Invalid("no process expectation".into()))?;
    if validation["reference_expectation_kind"] != "process-failure"
        || capture.invocation["executable_sha256"] != validation["actual_executable_sha256"]
        || capture.invocation["settings_sha256"] != validation["settings_sha256"]
        || capture.invocation["profile_sha256"] != validation["profile_sha256"]
        || capture.invocation["file_metadata_sha256"] != validation["file_metadata_sha256"]
    {
        return Err(Error::Invalid(
            "actual process identity is not the validated exact observation group".into(),
        ));
    }

    if capture.output.signal.is_some()
        || capture.output.status.0 != process.exit
        || capture.invocation["observer_no_deadline"] != true
    {
        return Err(Error::Invalid(
            "reference termination differs from declared completed failure".into(),
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
            "reference process bytes differ from declared expectation".into(),
        ));
    }
    let profile_root = rules
        .parent()
        .ok_or_else(|| Error::Invalid("missing process profile boundary".into()))?;
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
            file_metadata_sha256: validation["file_metadata_sha256"].as_str(),
            source_sha256: validation["source_sha256"]
                .as_str()
                .ok_or_else(|| Error::Invalid("missing validated source identity".into()))?,
            evidence: &[],
        })
        .map_err(|error| Error::Invalid(error.to_string()))?;
    Ok(())
}

/// Validate every archived observation and require an exact pair for the actual binary.
pub(super) fn validate_archives(
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    profile_root: &std::path::Path,
    profile: &crate::domain::suite::types::Tool,
    case: &crate::domain::suite::types::PreparedCase,
    identity: &crate::domain::suite::types::ReferenceDiscrepancyIdentity,
    actual: &crate::domain::suite::types::ArtifactDigest,
) -> Result<(), Error> {
    use crate::outbound::suite::types::{ArtifactPath, discrepancy::EvidenceArtifact};
    let process =
        expected(case).ok_or_else(|| Error::Invalid("missing process contract".into()))?;
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
                "process evidence has an unapproved exact binary".into(),
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
                    "process observations must retain separate raw artifacts".into(),
                ));
            }
            artifacts.push(EvidenceArtifact {
                path: artifact.path.clone(),
                sha256: artifact.sha256.0.clone(),
            });
        }
    }
    if groups.is_empty()
        || groups.values().any(|count| *count < 2)
        || groups.get(actual.0.as_str()).copied().unwrap_or(0) < 2
    {
        return Err(Error::Invalid(
            "process expectation needs two captures for the actual exact executable".into(),
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
                .ok_or_else(|| Error::Invalid("missing raw invocation".into()))?,
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let root = metadata["actual_request_root"]
            .as_str()
            .ok_or_else(|| Error::Invalid("archived request root is missing".into()))?;
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
        if !metadata["metadata_error"].is_null()
            || metadata["files"] != serde_json::json!(required_files)
            || metadata["executable_sha256"] != observation.executable_sha256.0
            || metadata["profile_sha256"] != identity.profile_sha256.0
            || metadata["source_sha256"] != identity.source_sha256.0
            || metadata["file_metadata_sha256"]
                != serde_json::json!(identity.file_metadata_sha256.as_ref().map(|value| &value.0))
            || metadata["settings_sha256"] != identity.settings_sha256.0
            || metadata["tool"] != profile.tool.0.as_str()
            || metadata["version"] != profile.version.0.as_str()
            || metadata["exit"].as_i64() != Some(i64::from(process.exit))
            || metadata.get("signal") != Some(&serde_json::Value::Null)
            || metadata["observer_no_deadline"] != true
            || bytes.get(&observation.stdout.path) != Some(&stdout)
            || bytes.get(&observation.stderr.path) != Some(&stderr)
        {
            return Err(Error::Invalid(
                "archived process bytes, termination or immutable identity differ".into(),
            ));
        }
        let mut archived_case = case.clone();
        archived_case.request.root = root.into();
        archived_case.request.state = metadata["state"]
            .as_str()
            .ok_or_else(|| Error::Invalid("archived state boundary is missing".into()))?
            .into();
        let arguments = serde_json::from_value::<Vec<String>>(metadata["arguments"].clone())
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let environment = serde_json::from_value::<std::collections::BTreeMap<String, String>>(
            metadata["environment"].clone(),
        )
        .map_err(|error| Error::Invalid(error.to_string()))?;
        let invocation = crate::domain::measurements::types::Invocation {
            executable: std::path::PathBuf::new(),
            arguments: arguments.into_iter().map(Into::into).collect(),
            environment: environment
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            directory: metadata["directory"]
                .as_str()
                .ok_or_else(|| Error::Invalid("archived invocation directory is missing".into()))?
                .into(),
        };
        if super::invocation_identity::normalized_invocation(&invocation, &archived_case, profile)?
            != metadata["normalized_settings"]
            || super::invocation_identity::invocation_settings(
                &invocation,
                &archived_case,
                profile,
            )? != identity.settings_sha256
        {
            return Err(Error::Invalid(
                "archived argv/environment do not establish the pinned harness settings".into(),
            ));
        }
    }
    Ok(())
}

/// Bind the chosen exact-binary archive group into both runtime and offline receipts.
pub(super) fn group_digest(
    process: &ProcessFailure,
    executable: &str,
) -> Result<crate::domain::suite::types::ArtifactDigest, Error> {
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
                    "process observation groups must retain distinct raw captures".into(),
                ));
            }
        }
        *groups.entry(&observation.executable_sha256.0).or_default() += 1;
    }
    if groups.values().any(|count| *count < 2) || groups.get(executable).copied().unwrap_or(0) < 2 {
        return Err(Error::Invalid(
            "missing reproducible exact-binary observation group".into(),
        ));
    }
    let mut rows = process
        .observations
        .iter()
        .filter(|row| row.executable_sha256.0 == executable)
        .map(|row| {
            serde_json::json!({
                "executable_sha256":row.executable_sha256.0,
                "invocation":{"path":row.invocation.path,"sha256":row.invocation.sha256.0},
                "stdout":{"path":row.stdout.path,"sha256":row.stdout.sha256.0},
                "stderr":{"path":row.stderr.path,"sha256":row.stderr.sha256.0},
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(serde_json::Value::to_string);
    Ok(super::discrepancy::json_digest(&serde_json::json!(rows)))
}
