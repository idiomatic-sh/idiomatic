//! Read and verify explicit artifacts without application or domain dependencies.
use crate::outbound::suite::adapters::local::constants;
use crate::outbound::suite::errors::SuiteError;
use crate::outbound::suite::types::{
    ArtifactMetadata, CaseArtifacts, CaseClaim, ClaimedEvidence, ClaimedProcess,
    EvidenceRequirement,
};
use ryo_benchmark_contracts::domain::documents::types::text::{CaseName, RuleName};
use sha2::{Digest as _, Sha256};
use std::path::{Component, Path};

/// Refuse platform-specific escape spellings before touching artifact paths.
fn relative(path: &Path) -> Result<(), SuiteError> {
    let text = path
        .to_str()
        .ok_or_else(|| SuiteError::Invalid(constants::NON_UTF8_ARTIFACT_PATH.into()))?;
    if text.is_empty()
        || text.contains(['\\', ':'])
        || !path
            .components()
            .all(|item| matches!(item, Component::Normal(_)))
    {
        return Err(SuiteError::Invalid(
            constants::UNSAFE_ARTIFACT_RELATIVE_PATH.into(),
        ));
    }
    Ok(())
}

/// Read the authored reference certificates; interpretation belongs to application orchestration.
pub(super) fn case_claims(
    profile: &Path,
    owners: &[ryo::linter::RuleId],
) -> Result<Vec<CaseClaim>, SuiteError> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(profile.join(constants::RULES))? {
        let entry = entry?;
        if !entry.path().join(constants::QUERY).is_file() {
            continue;
        }
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_bytes| SuiteError::Invalid(constants::NON_UTF8_RULE_DIRECTORY.into()))?;
        let rule = super::rule_cases::schemas::rule(&rule)
            .map_err(|error| SuiteError::Invalid(error.to_string()))?;
        if !owners.contains(&rule) {
            continue;
        }
        let text = std::fs::read_to_string(entry.path().join(constants::CASES))?;
        for (name, discrepancy, basis) in super::rule_cases::schemas::case_claims(&text)? {
            result.push(CaseClaim {
                rule: RuleName(rule.to_string()),
                name: CaseName(name),
                basis,
                discrepancy,
            });
        }
    }
    Ok(result)
}

/// Verify exact sources and evidence inside their separately explicit boundaries.
pub(super) fn verify(artifacts: CaseArtifacts<'_>) -> Result<(), SuiteError> {
    if source_digest(artifacts.root, artifacts.files)? != artifacts.source_sha256.0 {
        return Err(SuiteError::Invalid(
            constants::STALE_REFERENCE_DISCREPANCY_SOURCE_IDENTITY.into(),
        ));
    }
    if artifacts
        .file_metadata_files
        .iter()
        .any(|file| !artifacts.files.contains(file))
        || (artifacts.file_metadata == ArtifactMetadata::Undeclared
            && !artifacts.file_metadata_files.is_empty())
    {
        return Err(SuiteError::Invalid(
            constants::UNDECLARED_REFERENCE_DISCREPANCY_OBSERVATION.into(),
        ));
    }
    if let ArtifactMetadata::Declared(expected) = artifacts.file_metadata {
        let actual =
            super::input_metadata::actual_digest(artifacts.root, artifacts.file_metadata_files)
                .map_err(|error| SuiteError::Invalid(error.to_string()))?;
        if actual != expected.0 {
            return Err(SuiteError::Invalid(
                constants::STALE_REFERENCE_DISCREPANCY_FILE_METADATA_ID.into(),
            ));
        }
    }
    let boundary = artifacts.profile.canonicalize()?;
    for evidence in artifacts.evidence {
        relative(&evidence.path)?;
        let path = boundary.join(&evidence.path).canonicalize()?;
        if !path.starts_with(&boundary) {
            return Err(SuiteError::Invalid(
                constants::STALE_OR_EXTERNAL_DISCREPANCY_EVIDENCE.into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != evidence.sha256.0 {
            return Err(SuiteError::Invalid(
                constants::STALE_DISCREPANCY_EVIDENCE_BYTES.into(),
            ));
        }
        verify_requirement(&bytes, &evidence.requirement)?;
    }
    Ok(())
}

/// Check the asserted source and prior execution against the hash-verified envelope bytes.
pub(super) fn verify_requirement(
    bytes: &[u8],
    requirement: &EvidenceRequirement,
) -> Result<(), SuiteError> {
    use serde_json::Value;
    let fail =
        || SuiteError::Invalid(constants::TOOL_CONTRACT_EVIDENCE_DOES_NOT_ESTABLISH.into());
    let value = match requirement {
        EvidenceRequirement::Language => return Ok(()),
        _ => serde_json::from_slice::<Value>(bytes).map_err(|_| fail())?,
    };
    let field = |name: &str| value.get(name).and_then(Value::as_str).ok_or_else(fail);
    match requirement {
        EvidenceRequirement::Language => Ok(()),
        EvidenceRequirement::PublicToolContract {
            source_revision,
            source_path,
        } => {
            if field(constants::SCHEMA)? != constants::PUBLIC_TOOL_CONTRACT_V1
                || field(constants::SOURCE_REVISION)? != source_revision.0
                || field(constants::SOURCE_PATH)? != source_path.0
            {
                return Err(fail());
            }
            let source = field(constants::SOURCE_TEXT)?;
            let assertion = field(constants::CONTRACT_ASSERTION)?;
            if assertion.is_empty()
                || !source.contains(assertion)
                || field(constants::SOURCE_SHA256)?
                    != format!("{:x}", Sha256::digest(source.as_bytes()))
            {
                return Err(fail());
            }
            Ok(())
        }
        EvidenceRequirement::PriorWorkerRun {
            worker_sha256,
            control_source_sha256,
            control_query_sha256,
            control_profile_sha256,
            witness_case,
        } => {
            if field(constants::SCHEMA)? != constants::PRIOR_WORKER_CONTROL_V1
                || field(constants::WORKER_SHA256)? != worker_sha256.0
                || field(constants::CONTROL_SOURCE_SHA256)? != control_source_sha256.0
                || field(constants::CONTROL_QUERY_SHA256)? != control_query_sha256.0
                || field(constants::CONTROL_PROFILE_SHA256)? != control_profile_sha256.0
                || field(constants::WITNESS_CASE)? != witness_case.0
            {
                return Err(fail());
            }
            let query = field(constants::CONTROL_QUERY)?;
            let profile = field(constants::CONTROL_PROFILE)?;
            if format!("{:x}", Sha256::digest(query.as_bytes())) != control_query_sha256.0
                || format!("{:x}", Sha256::digest(profile.as_bytes())) != control_profile_sha256.0
            {
                return Err(fail());
            }
            let sources = value
                .get(constants::CONTROL_SOURCES)
                .and_then(Value::as_object)
                .ok_or_else(fail)?;
            let mut digest = Sha256::new();
            let mut names = sources.keys().collect::<Vec<_>>();
            names.sort();
            for name in names {
                let source = sources[name].as_str().ok_or_else(fail)?;
                digest.update(name.as_bytes());
                digest.update([0]);
                digest.update(source.as_bytes());
            }
            if format!("{:x}", digest.finalize()) != control_source_sha256.0 {
                return Err(fail());
            }
            let receipt = field(constants::RAW_RECEIPT)?;
            if field(constants::RAW_RECEIPT_SHA256)?
                != format!("{:x}", Sha256::digest(receipt.as_bytes()))
            {
                return Err(fail());
            }
            // The control receipt alone does not name the executable or staged query.
            // Require the captured execution manifest from that run to bind all inputs.
            let execution = field(constants::EXECUTION_MANIFEST)?;
            if field(constants::EXECUTION_MANIFEST_SHA256)?
                != format!("{:x}", Sha256::digest(execution.as_bytes()))
            {
                return Err(fail());
            }
            let run: Value = serde_json::from_str(execution).map_err(|_| fail())?;
            if run[constants::SCHEMA] != constants::PRIOR_WORKER_EXECUTION_V1
                || run[constants::WORKER_SHA256] != worker_sha256.0
                || run[constants::CONTROL_SOURCE_SHA256] != control_source_sha256.0
                || run[constants::CONTROL_QUERY_SHA256] != control_query_sha256.0
                || run[constants::CONTROL_PROFILE_SHA256] != control_profile_sha256.0
                || run[constants::RAW_RECEIPT_SHA256] != field(constants::RAW_RECEIPT_SHA256)?
                || run[constants::WITNESS_CASE] != witness_case.0
                || !matches!(run[constants::STATUS].as_i64(), Some(0 | 1))
                || run[constants::COMMAND][1] != constants::VERIFY_CASES
                || run[constants::COMMAND]
                    .as_array()
                    .is_none_or(|parts| parts.len() != 4)
                || run[constants::COMMAND][3] != run[constants::STAGE_ROOT]
                || run[constants::COMMAND][0]
                    .as_str()
                    .is_none_or(|path| !path.ends_with(worker_sha256))
            {
                return Err(fail());
            }
            let mut witnessed = false;
            for line in receipt.lines() {
                let row: Value = serde_json::from_str(line).map_err(|_| fail())?;
                if row[constants::SIDE] == constants::CANDIDATE
                    && row[constants::NAME] == witness_case.0
                {
                    if witnessed
                        || row[constants::COMPLETE] != true
                        || row[constants::ERROR_2] != Value::Null
                        || row[constants::CASE_PASSED] != true
                        || row[constants::SOURCE_SHA256] != control_source_sha256.0
                        || row[constants::SKIPPED] != serde_json::json!([])
                        || row[constants::DIAGNOSTICS] != row[constants::EXPECTED]
                        || row[constants::DIAGNOSTICS]
                            .as_array()
                            .is_none_or(Vec::is_empty)
                        || row[constants::COVERAGE][constants::SELECTED_RULE_FILE_PAIRS]
                            .as_u64()
                            .is_none_or(|number| {
                                number == 0
                                    || row[constants::COVERAGE]
                                        [constants::EVALUATED_RULE_FILE_PAIRS]
                                        .as_u64()
                                        != Some(number)
                            })
                        || row[constants::COVERAGE][constants::UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS]
                            != 0
                        || row[constants::COVERAGE][constants::SKIPPED_RULE_FILE_PAIRS] != 0
                    {
                        return Err(fail());
                    }
                    witnessed = true;
                }
            }
            if !witnessed {
                return Err(fail());
            }
            Ok(())
        }
    }
}

/// Snapshot only explicitly declared evidence, preserving exact bytes.
pub(super) fn copy_evidence(
    profile: &Path,
    destination: &Path,
    cases: &Path,
) -> std::io::Result<()> {
    let claims = super::rule_cases::schemas::case_claims(&std::fs::read_to_string(cases)?)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let boundary = profile.canonicalize()?;
    for (_, claim, _) in claims {
        let mut artifacts = claim
            .evidence
            .iter()
            .map(|item| {
                let (ClaimedEvidence::LanguageSpec { path, sha256, .. }
                | ClaimedEvidence::IndependentLanguageRun { path, sha256, .. }
                | ClaimedEvidence::PublicToolContract { path, sha256, .. }
                | ClaimedEvidence::PriorWorkerRun { path, sha256, .. }) = item;
                (path.clone(), sha256.0.clone())
            })
            .collect::<Vec<_>>();
        if let ClaimedProcess::Failure(failure) = &claim.discrepancy.process {
            for observation in &failure.observations {
                for item in [&observation.invocation, &observation.stdout, &observation.stderr] {
                    artifacts.push((item.path.clone(), item.sha256.0.clone()));
                }
            }
        }
        for (path, expected) in artifacts {
            relative(&path)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let requested = boundary.join(&path);
            let contextual = |error: std::io::Error| {
                std::io::Error::new(
                    error.kind(),
                    format!("case evidence {}: {error}", requested.display()),
                )
            };
            let source = requested.canonicalize().map_err(contextual)?;
            let bytes = std::fs::read(&source).map_err(contextual)?;
            if !source.starts_with(&boundary)
                || format!("{:x}", Sha256::digest(&bytes)) != expected
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    constants::STALE_OR_EXTERNAL_DISCREPANCY_EVIDENCE,
                ));
            }
            let target = destination.join(&path);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(target, bytes)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "discrepancy_artifacts_test.rs"]
mod tests;

/// Read exact declared raw bytes under the profile boundary after validating their digests.
pub(super) fn read(
    profile: &Path,
    artifacts: &[crate::outbound::suite::types::EvidenceArtifact],
) -> Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>, SuiteError> {
    let boundary = profile.canonicalize()?;
    let mut result = std::collections::BTreeMap::new();
    for artifact in artifacts {
        relative(&artifact.path)?;
        let path = boundary.join(&artifact.path).canonicalize()?;
        if !path.starts_with(&boundary) || !path.is_file() {
            return Err(SuiteError::Invalid(
                constants::RAW_ARTIFACT_ESCAPES_ITS_PROFILE.into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256 {
            return Err(SuiteError::Invalid(
                constants::RAW_ARTIFACT_BYTE_IDENTITY_DIFFERS.into(),
            ));
        }
        if result.insert(artifact.path.clone(), bytes).is_some() {
            return Err(SuiteError::Invalid(
                constants::DUPLICATE_RAW_ARTIFACT_PATH.into(),
            ));
        }
    }
    Ok(result)
}

/// Canonical source framing shared by fresh observations and discrepancy evidence.
pub(super) fn source_digest(
    root: &Path,
    files: &[std::path::PathBuf],
) -> Result<String, SuiteError> {
    let root = root.canonicalize()?;
    let mut files = files.to_vec();
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    let mut source = Sha256::new();
    for file in files {
        relative(&file)?;
        let path = root.join(&file).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(SuiteError::Invalid(
                constants::SOURCE_ARTIFACT_ESCAPES_ITS_BOUNDARY.into(),
            ));
        }
        source.update(
            file.to_str()
                .ok_or_else(|| SuiteError::Invalid(constants::NONUTF8_SOURCE_PATH.into()))?
                .as_bytes(),
        );
        source.update([0]);
        source.update(std::fs::read(path)?);
    }
    Ok(format!("{:x}", source.finalize()))
}
