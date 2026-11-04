//! Filesystem encoding and atomic publication of complete inventory snapshots.
use crate::outbound::inventory::{
    interface::InventoryPort,
    types::{CapturedObservation, Registration, Row, Summary},
};
use std::{io::Write as _, path::Path};

/// Decode the adapter's generated native invocation registration.
fn load(path: &Path) -> Result<Registration, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
/// Publish a whole snapshot atomically, retaining the previous valid snapshot on failure.
fn publish(output: &Path, summary: &Summary) -> Result<(), String> {
    std::fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(output).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut temporary, summary).map_err(|e| e.to_string())?;
    temporary.write_all(b"\n").map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    let _persisted = temporary
        .persist(output.join("summary.json"))
        .map_err(|e| e.to_string())?;
    Ok(())
}
/// Keep the exact child streams before interpreting its result.
fn streams(directory: &Path, stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("stdout.bin"), stdout).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("stderr.bin"), stderr).map_err(|e| e.to_string())
}
/// Read actual per-side observations; absence and malformed records remain errors.
fn observations(path: &Path) -> Result<Vec<serde_json::Value>, String> {
    std::fs::read_to_string(path)
        .map_err(|e| e.to_string())?
        .lines()
        .map(|line| serde_json::from_str(line).map_err(|e| e.to_string()))
        .collect()
}
/// Persist one terminal row beside its raw receipts.
fn result(directory: &Path, row: &Row) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(row).map_err(|e| e.to_string())?;
    std::fs::write(directory.join("result.json"), bytes).map_err(|e| e.to_string())
}

/// Refuse stale result reuse while allowing the provider's already staged reproductions.
fn begin(output: &Path) -> Result<(), String> {
    if output.join("summary.json").exists() {
        return Err("inventory output already contains a summary".into());
    }
    std::fs::create_dir_all(output).map_err(|e| e.to_string())
}
/// Each invocation owns fresh native case receipts.
fn prepare(directory: &Path) -> Result<(), String> {
    if ["cases.jsonl", "stdout.bin", "stderr.bin", "result.json"]
        .iter()
        .any(|name| directory.join(name).exists())
    {
        return Err("inventory rule output already contains receipts".into());
    }
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())
}

/// Hash an actual receipt's exact bytes at the filesystem boundary.
fn digest(path: &Path) -> Result<String, String> {
    use sha2::Digest as _;
    Ok(format!(
        "{:x}",
        sha2::Sha256::digest(std::fs::read(path).map_err(|e| e.to_string())?)
    ))
}

/// Local registrations and atomic verification artifacts.
pub(crate) struct InventoryLocalAdapter {
    suite: std::sync::Arc<dyn crate::outbound::suite::interface::SuitePort>,
}
impl InventoryLocalAdapter {
    pub(crate) fn new(
        suite: std::sync::Arc<dyn crate::outbound::suite::interface::SuitePort>,
    ) -> Self {
        Self { suite }
    }
}

impl InventoryPort for InventoryLocalAdapter {
    fn canonical(
        &self,
        authority: &crate::outbound::inventory::types::RegistrationAuthority,
    ) -> Result<crate::outbound::inventory::types::CanonicalFacts, String> {
        use crate::outbound::inventory::types::{CanonicalFacts, RegistrationAuthority};
        use crate::outbound::suite::types::ArtifactPath;
        let RegistrationAuthority::Execution {
            profile,
            reference,
            worker,
            ..
        } = authority
        else {
            return Err("receipt-only registration cannot execute".into());
        };
        let tool = self
            .suite
            .profile(ArtifactPath(profile))
            .map_err(|e| format!("canonical profile {}: {e}", profile.display()))?;
        let read = |path: &Path| {
            self.suite
                .digest(ArtifactPath(path))
                .map(|d| d.0)
                .map_err(|e| format!("inventory artifact {}: {e}", path.display()))
        };
        Ok(CanonicalFacts {
            profile_sha256: read(&profile.join("benchmark.toml"))?,
            reference_sha256: read(&reference.path)?,
            worker_sha256: read(&worker.path)?,
            defaults: tool.defaults.iter().map(ToString::to_string).collect(),
            deferred: tool.deferred.keys().map(ToString::to_string).collect(),
            pending: tool.pending.keys().map(ToString::to_string).collect(),
            approved_reference: tool.sha256.iter().map(|s| s.0.clone()).collect(),
        })
    }
    fn verify_artifacts(
        &self,
        artifacts: &std::collections::BTreeMap<std::path::PathBuf, String>,
    ) -> Result<(), String> {
        if artifacts.is_empty() {
            return Err("registered case context is empty".into());
        }
        for (path, expected) in artifacts {
            let actual = self
                .suite
                .digest(crate::outbound::suite::types::ArtifactPath(path))
                .map_err(|e| format!("registered case artifact {}: {e}", path.display()))?;
            if actual.0 != *expected {
                return Err(format!(
                    "registered case artifact changed: {}",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    fn observations(&self, receipt: &Path) -> Result<CapturedObservation, String> {
        let records = observations(receipt)?;
        Ok(CapturedObservation {
            artifacts: super::case_artifacts::read(receipt, &records)?,
            records,
            sha256: Some(digest(receipt)?),
        })
    }

    fn load(&self, path: &Path) -> Result<Registration, String> {
        load(path)
    }
    fn begin(&self, output: &Path) -> Result<(), String> {
        begin(output)
    }
    fn publish(&self, output: &Path, summary: &Summary) -> Result<(), String> {
        publish(output, summary)
    }
    fn prepare(&self, directory: &Path) -> Result<(), String> {
        prepare(directory)
    }
    fn capture(
        &self,
        directory: &Path,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Result<CapturedObservation, String> {
        streams(directory, stdout, stderr)?;
        let path = directory.join("cases.jsonl");
        if !path.is_file() {
            return Ok(CapturedObservation {
                artifacts: std::collections::BTreeMap::new(),
                records: Vec::new(),
                sha256: None,
            });
        }
        self.observations(&path)
    }
    fn result(&self, directory: &Path, row: &Row) -> Result<(), String> {
        result(directory, row)
    }
}

impl std::fmt::Debug for InventoryLocalAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InventoryLocalAdapter")
            .finish_non_exhaustive()
    }
}
