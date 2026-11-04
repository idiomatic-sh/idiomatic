//! Read and verify explicit artifacts without application or domain dependencies.
use crate::outbound::suite::types::discrepancy::{
    ArtifactError, CaseArtifacts, CaseReferenceWire, EvidenceRequirement,
};
use sha2::{Digest as _, Sha256};
use std::path::{Component, Path};

/// Refuse platform-specific escape spellings before touching artifact paths.
fn relative(path: &Path) -> Result<(), ArtifactError> {
    let text = path
        .to_str()
        .ok_or_else(|| ArtifactError::Invalid("non-UTF8 artifact path".into()))?;
    if text.is_empty()
        || text.contains(['\\', ':'])
        || !path
            .components()
            .all(|item| matches!(item, Component::Normal(_)))
    {
        return Err(ArtifactError::Invalid(
            "unsafe artifact-relative path".into(),
        ));
    }
    Ok(())
}

/// Read raw optional reference expectations; interpretation belongs to application orchestration.
pub(super) fn case_reference_wires(
    profile: &Path,
    tool: &crate::domain::suite::types::Tool,
) -> Result<Vec<CaseReferenceWire>, ArtifactError> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir(profile.join("rules"))? {
        let entry = entry?;
        if !entry.path().join("query.ryoql").is_file() {
            continue;
        }
        let rule = entry
            .file_name()
            .into_string()
            .map_err(|_bytes| ArtifactError::Invalid("non-UTF8 rule directory".into()))?;
        let rule = super::schemas::rule(&rule)
            .map_err(|error| ArtifactError::Invalid(error.to_string()))?;
        if !crate::domain::suite::service::owns_cases(tool, &rule) {
            continue;
        }
        let text = std::fs::read_to_string(entry.path().join("cases.toml"))?;
        for (name, discrepancy, basis) in super::schemas::case_reference_wires(&text)? {
            result.push(CaseReferenceWire {
                rule: rule.to_string(),
                name,
                basis,
                discrepancy,
            });
        }
    }
    Ok(result)
}

/// Verify exact sources and evidence inside their separately explicit boundaries.
pub(super) fn verify(artifacts: CaseArtifacts<'_>) -> Result<(), ArtifactError> {
    if source_digest(artifacts.root, artifacts.files)? != artifacts.source_sha256 {
        return Err(ArtifactError::Invalid(
            "stale reference discrepancy source identity".into(),
        ));
    }
    if artifacts
        .file_metadata_files
        .iter()
        .any(|file| !artifacts.files.contains(file))
        || (artifacts.file_metadata_sha256.is_none() && !artifacts.file_metadata_files.is_empty())
    {
        return Err(ArtifactError::Invalid(
            "undeclared reference discrepancy observation scope".into(),
        ));
    }
    if let Some(expected) = artifacts.file_metadata_sha256 {
        let actual =
            super::input_metadata::actual_digest(artifacts.root, artifacts.file_metadata_files)
                .map_err(|error| ArtifactError::Invalid(error.to_string()))?;
        if actual != expected {
            return Err(ArtifactError::Invalid(
                "stale reference discrepancy file metadata identity".into(),
            ));
        }
    }
    let boundary = artifacts.profile.canonicalize()?;
    for evidence in artifacts.evidence {
        relative(&evidence.path)?;
        let path = boundary.join(&evidence.path).canonicalize()?;
        if !path.starts_with(&boundary) {
            return Err(ArtifactError::Invalid(
                "stale or external discrepancy evidence".into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != evidence.sha256 {
            return Err(ArtifactError::Invalid(
                "stale discrepancy evidence bytes".into(),
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
) -> Result<(), ArtifactError> {
    use serde_json::Value;
    let fail = || {
        ArtifactError::Invalid(
            "tool-contract evidence does not establish its declared authority".into(),
        )
    };
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
            if field("schema")? != "public-tool-contract/v1"
                || field("source_revision")? != source_revision
                || field("source_path")? != source_path
            {
                return Err(fail());
            }
            let source = field("source_text")?;
            let assertion = field("contract_assertion")?;
            if assertion.is_empty()
                || !source.contains(assertion)
                || field("source_sha256")? != format!("{:x}", Sha256::digest(source.as_bytes()))
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
            if field("schema")? != "prior-worker-control/v1"
                || field("worker_sha256")? != worker_sha256
                || field("control_source_sha256")? != control_source_sha256
                || field("control_query_sha256")? != control_query_sha256
                || field("control_profile_sha256")? != control_profile_sha256
                || field("witness_case")? != witness_case
            {
                return Err(fail());
            }
            let query = field("control_query")?;
            let profile = field("control_profile")?;
            if format!("{:x}", Sha256::digest(query.as_bytes())) != *control_query_sha256
                || format!("{:x}", Sha256::digest(profile.as_bytes())) != *control_profile_sha256
            {
                return Err(fail());
            }
            let sources = value
                .get("control_sources")
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
            if format!("{:x}", digest.finalize()) != *control_source_sha256 {
                return Err(fail());
            }
            let receipt = field("raw_receipt")?;
            if field("raw_receipt_sha256")? != format!("{:x}", Sha256::digest(receipt.as_bytes())) {
                return Err(fail());
            }
            // The control receipt alone does not name the executable or staged query.
            // Require the captured execution manifest from that run to bind all inputs.
            let execution = field("execution_manifest")?;
            if field("execution_manifest_sha256")?
                != format!("{:x}", Sha256::digest(execution.as_bytes()))
            {
                return Err(fail());
            }
            let run: Value = serde_json::from_str(execution).map_err(|_| fail())?;
            if run["schema"] != "prior-worker-execution/v1"
                || run["worker_sha256"] != *worker_sha256
                || run["control_source_sha256"] != *control_source_sha256
                || run["control_query_sha256"] != *control_query_sha256
                || run["control_profile_sha256"] != *control_profile_sha256
                || run["raw_receipt_sha256"] != field("raw_receipt_sha256")?
                || run["witness_case"] != *witness_case
                || !matches!(run["status"].as_i64(), Some(0 | 1))
                || run["command"][1] != "verify-cases"
                || run["command"]
                    .as_array()
                    .is_none_or(|parts| parts.len() != 4)
                || run["command"][3] != run["stage_root"]
                || run["command"][0]
                    .as_str()
                    .is_none_or(|path| !path.ends_with(worker_sha256))
            {
                return Err(fail());
            }
            let mut witnessed = false;
            for line in receipt.lines() {
                let row: Value = serde_json::from_str(line).map_err(|_| fail())?;
                if row["side"] == "candidate" && row["name"] == *witness_case {
                    if witnessed
                        || row["complete"] != true
                        || row["error"] != Value::Null
                        || row["case_passed"] != true
                        || row["source_sha256"] != *control_source_sha256
                        || row["skipped"] != serde_json::json!([])
                        || row["diagnostics"] != row["expected"]
                        || row["diagnostics"].as_array().is_none_or(Vec::is_empty)
                        || row["coverage"]["selected_rule_file_pairs"]
                            .as_u64()
                            .is_none_or(|number| {
                                number == 0
                                    || row["coverage"]["evaluated_rule_file_pairs"].as_u64()
                                        != Some(number)
                            })
                        || row["coverage"]["unknown_or_unread_rule_file_pairs"] != 0
                        || row["coverage"]["skipped_rule_file_pairs"] != 0
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
    let wires = super::schemas::case_reference_wires(&std::fs::read_to_string(cases)?)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let boundary = profile.canonicalize()?;
    for (_, wire, _) in wires {
        let mut artifacts = wire
            .evidence
            .into_iter()
            .map(
                |item| crate::outbound::suite::types::discrepancy::EvidenceArtifact {
                    path: item.artifact().0.to_path_buf(),
                    sha256: item.artifact().1.to_owned(),
                    requirement: item.requirement(),
                },
            )
            .collect::<Vec<_>>();
        if let Some(crate::outbound::suite::types::process::ProcessFailureWire::ProcessFailure {
            observations,
            ..
        }) = wire.process
        {
            for observation in observations {
                for item in [
                    observation.invocation,
                    observation.stdout,
                    observation.stderr,
                ] {
                    artifacts.push(
                        crate::outbound::suite::types::discrepancy::EvidenceArtifact {
                            path: item.path,
                            sha256: item.sha256,
                            requirement: crate::outbound::suite::types::discrepancy::EvidenceRequirement::Language,
                        },
                    );
                }
            }
        }
        for evidence in artifacts {
            relative(&evidence.path)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let requested = boundary.join(&evidence.path);
            let contextual = |error: std::io::Error| {
                std::io::Error::new(
                    error.kind(),
                    format!("case evidence {}: {error}", requested.display()),
                )
            };
            let source = requested.canonicalize().map_err(contextual)?;
            let bytes = std::fs::read(&source).map_err(contextual)?;
            if !source.starts_with(&boundary)
                || format!("{:x}", Sha256::digest(&bytes)) != evidence.sha256
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "stale or external discrepancy evidence",
                ));
            }
            let target = destination.join(&evidence.path);
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
    artifacts: &[crate::outbound::suite::types::discrepancy::EvidenceArtifact],
) -> Result<std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>, ArtifactError> {
    let boundary = profile.canonicalize()?;
    let mut result = std::collections::BTreeMap::new();
    for artifact in artifacts {
        relative(&artifact.path)?;
        let path = boundary.join(&artifact.path).canonicalize()?;
        if !path.starts_with(&boundary) || !path.is_file() {
            return Err(ArtifactError::Invalid(
                "raw artifact escapes its profile".into(),
            ));
        }
        let bytes = std::fs::read(path)?;
        if format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256 {
            return Err(ArtifactError::Invalid(
                "raw artifact byte identity differs".into(),
            ));
        }
        if result.insert(artifact.path.clone(), bytes).is_some() {
            return Err(ArtifactError::Invalid("duplicate raw artifact path".into()));
        }
    }
    Ok(result)
}

/// Canonical source framing shared by fresh observations and discrepancy evidence.
pub(super) fn source_digest(
    root: &Path,
    files: &[std::path::PathBuf],
) -> Result<String, ArtifactError> {
    let root = root.canonicalize()?;
    let mut files = files.to_vec();
    files.sort_by(|left, right| left.to_string_lossy().cmp(&right.to_string_lossy()));
    let mut source = Sha256::new();
    for file in files {
        relative(&file)?;
        let path = root.join(&file).canonicalize()?;
        if !path.starts_with(&root) {
            return Err(ArtifactError::Invalid(
                "source artifact escapes its boundary".into(),
            ));
        }
        source.update(
            file.to_str()
                .ok_or_else(|| ArtifactError::Invalid("nonUTF8 source path".into()))?
                .as_bytes(),
        );
        source.update([0]);
        source.update(std::fs::read(path)?);
    }
    Ok(format!("{:x}", source.finalize()))
}
