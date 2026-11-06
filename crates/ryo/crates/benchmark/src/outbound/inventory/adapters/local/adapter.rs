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
    temporary
        .write_all(crate::outbound::inventory::adapters::local::constants::NEWLINE)
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    let _persisted = temporary
        .persist(output.join(crate::outbound::inventory::adapters::local::constants::SUMMARY_JSON))
        .map_err(|e| e.to_string())?;
    Ok(())
}
/// Keep the exact child streams before interpreting its result.
fn streams(directory: &Path, stdout: &[u8], stderr: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join(crate::outbound::inventory::adapters::local::constants::STDOUT_BIN),
        stdout,
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(
        directory.join(crate::outbound::inventory::adapters::local::constants::STDERR_BIN),
        stderr,
    )
    .map_err(|e| e.to_string())
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
    std::fs::write(
        directory.join(crate::outbound::inventory::adapters::local::constants::RESULT_JSON),
        bytes,
    )
    .map_err(|e| e.to_string())
}

/// Refuse stale result reuse while allowing the provider's already staged reproductions.
fn begin(output: &Path) -> Result<(), String> {
    if output
        .join(crate::outbound::inventory::adapters::local::constants::SUMMARY_JSON)
        .exists()
    {
        return Err(crate::outbound::inventory::adapters::local::constants::INVENTORY_OUTPUT_ALREADY_CONTAINS_A_SUMMARY.into());
    }
    std::fs::create_dir_all(output).map_err(|e| e.to_string())
}
/// Each invocation owns fresh native case receipts.
fn prepare(directory: &Path) -> Result<(), String> {
    if [
        crate::outbound::inventory::adapters::local::constants::CASES_JSONL,
        crate::outbound::inventory::adapters::local::constants::STDOUT_BIN,
        crate::outbound::inventory::adapters::local::constants::STDERR_BIN,
        crate::outbound::inventory::adapters::local::constants::RESULT_JSON,
    ]
    .iter()
    .any(|name| directory.join(name).exists())
    {
        return Err(crate::outbound::inventory::adapters::local::constants::INVENTORY_RULE_OUTPUT_ALREADY_CONTAINS_RECEI.into());
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
            return Err(crate::outbound::inventory::adapters::local::constants::RECEIPT_ONLY_REGISTRATION_CANNOT_EXECUTE.into());
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
            profile_sha256: read(
                &profile
                    .join(crate::outbound::inventory::adapters::local::constants::BENCHMARK_TOML),
            )?,
            reference_sha256: read(&reference.path)?,
            worker_sha256: read(&worker.path)?,
            defaults: tool.defaults.iter().map(ToString::to_string).collect(),
            deferred: tool.deferred.keys().map(ToString::to_string).collect(),
            pending: tool.pending.keys().map(ToString::to_string).collect(),
            approved_reference: tool.sha256.iter().map(|s| s.0.clone()).collect(),
        })
    }
    fn inspect_context(
        &self,
        context: &Path,
        artifacts: &std::collections::BTreeMap<std::path::PathBuf, String>,
    ) -> Result<crate::outbound::inventory::types::ContextFacts, String> {
        if artifacts.is_empty() {
            return Err(crate::outbound::inventory::adapters::local::constants::REGISTERED_CASE_CONTEXT_IS_EMPTY.into());
        }
        let root = context
            .canonicalize()
            .map_err(|e| format!("registered context {}: {e}", context.display()))?;
        if root != context {
            return Err(crate::outbound::inventory::adapters::local::constants::REGISTERED_CONTEXT_MUST_BE_CANONICAL_AND.into());
        }
        let mut pending = vec![root.clone()];
        let mut files = std::collections::BTreeSet::new();
        while let Some(directory) = pending.pop() {
            for entry in std::fs::read_dir(directory).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let kind = entry.file_type().map_err(|e| e.to_string())?;
                if kind.is_dir() {
                    pending.push(entry.path());
                } else if kind.is_file() {
                    files.insert(entry.path());
                } else {
                    return Err(crate::outbound::inventory::adapters::local::constants::REGISTERED_CONTEXT_CONTAINS_NON_REGULAR_ENTR.into());
                }
            }
        }
        if files != artifacts.keys().cloned().collect() {
            return Err(crate::outbound::inventory::adapters::local::constants::REGISTERED_CONTEXT_NAMESPACE_DIFFERS_FROM_PI.into());
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
        let tool = self
            .suite
            .profile(crate::outbound::suite::types::ArtifactPath(context))
            .map_err(|error| {
                format!("registered context profile {}: {error}", context.display())
            })?;
        Ok(crate::outbound::inventory::types::ContextFacts {
            modules: tool.query_modules,
        })
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
        let path =
            directory.join(crate::outbound::inventory::adapters::local::constants::CASES_JSONL);
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
        f.debug_struct(
            crate::outbound::inventory::adapters::local::constants::INVENTORYLOCALADAPTER,
        )
        .finish_non_exhaustive()
    }
}
