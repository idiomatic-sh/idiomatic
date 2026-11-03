//! Read and verify explicit artifacts without application or domain dependencies.
use crate::outbound::suite::types::discrepancy::{ArtifactError, CaseArtifacts, CaseReferenceWire};
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
        let text = std::fs::read_to_string(entry.path().join("cases.toml"))?;
        for (name, discrepancy) in super::schemas::case_reference_wires(&text)? {
            result.push(CaseReferenceWire {
                rule: rule.clone(),
                name,
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
    let boundary = artifacts.profile.canonicalize()?;
    for evidence in artifacts.evidence {
        relative(&evidence.path)?;
        let path = boundary.join(&evidence.path).canonicalize()?;
        if !path.starts_with(&boundary)
            || format!("{:x}", Sha256::digest(std::fs::read(path)?)) != evidence.sha256
        {
            return Err(ArtifactError::Invalid(
                "stale or external discrepancy evidence".into(),
            ));
        }
    }
    Ok(())
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
    for (_, wire) in wires {
        let mut artifacts = wire
            .evidence
            .into_iter()
            .map(
                |item| crate::outbound::suite::types::discrepancy::EvidenceArtifact {
                    path: item.path,
                    sha256: item.sha256,
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
                        },
                    );
                }
            }
        }
        for evidence in artifacts {
            relative(&evidence.path)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            let source = boundary.join(&evidence.path).canonicalize()?;
            let bytes = std::fs::read(&source)?;
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
