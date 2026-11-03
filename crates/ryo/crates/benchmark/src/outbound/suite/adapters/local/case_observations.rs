//! Receipt-file scoped process artifacts; resumes never overwrite historical attempts.
use crate::outbound::suite::types::{discrepancy::ArtifactError, process::CapturedProcess};
use sha2::{Digest as _, Sha256};
use std::{io::Write as _, path::Path};

/// Persist one exact observation, adding storage-only raw artifact paths/digests.
pub(super) fn record(
    receipt: &Path,
    mut record: serde_json::Value,
    process: Option<CapturedProcess<'_>>,
) -> Result<(), ArtifactError> {
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| ArtifactError::Invalid("receipt filename must be UTF8".into()))?;
    let parent = receipt
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if name.contains(['\\', ':'])
        || std::fs::symlink_metadata(receipt).is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(ArtifactError::Invalid(
            "unsafe receipt file identity".into(),
        ));
    }
    std::fs::create_dir_all(parent)?;
    let boundary = parent.canonicalize()?;
    if let Some(process) = process {
        if process.key.len() != 64
            || !process
                .key
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ArtifactError::Invalid(
                "invalid framed process case key".into(),
            ));
        }
        // Kept directories survive incomplete receipts; only referenced attempts are loaded later.
        let directory = tempfile::Builder::new()
            .prefix(&format!("{name}.process-"))
            .tempdir_in(&boundary)?
            .keep();
        let case = directory.join(process.key);
        std::fs::create_dir(&case)?;
        let invocation = serde_json::to_vec(process.invocation)
            .map_err(|error| ArtifactError::Invalid(error.to_string()))?;
        let mut artifacts = serde_json::Map::new();
        for (label, filename, bytes) in [
            ("stdout", "stdout.bin", process.stdout),
            ("stderr", "stderr.bin", process.stderr),
            ("invocation", "invocation.json", invocation.as_slice()),
        ] {
            let path = case.join(filename);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(bytes)?;
            file.sync_data()?;
            let relative = path
                .strip_prefix(&boundary)
                .map_err(|error| ArtifactError::Invalid(error.to_string()))?;
            let relative = relative
                .components()
                .map(|part| {
                    part.as_os_str().to_str().ok_or_else(|| {
                        ArtifactError::Invalid("raw artifact path is not UTF8".into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("/");
            artifacts.insert(label.into(), serde_json::json!({"path":relative, "sha256":format!("{:x}", Sha256::digest(bytes))}));
        }
        record["process_observation"]["artifacts"] = serde_json::Value::Object(artifacts);
    }
    let mut bytes =
        serde_json::to_vec(&record).map_err(|error| ArtifactError::Invalid(error.to_string()))?;
    bytes.push(b'\n');
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(receipt)?;
    output.write_all(&bytes)?;
    output.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outbound::{
        inventory::interface::InventoryPort as _,
        suite::{interface::SuitePort as _, types::ArtifactPath},
    };
    fn write(receipt: &Path, stdout: &[u8]) {
        crate::outbound::suite::factory::local().record_case(ArtifactPath(receipt),serde_json::json!({"rule":"CHECK001","name":"one","side":"reference","process_observation":{"kind":"process-failure"}}),Some(CapturedProcess { key:&"a".repeat(64), invocation:&serde_json::json!({"exit":2}),stdout,stderr:b"error\n" })).unwrap();
    }
    #[test]
    fn raw_bytes_attempts_and_unreferenced_resume_history_are_retained() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("001.cases.jsonl");
        write(&receipt, &[255, 0, 1]);
        let first = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = first.records[0]["process_observation"]["artifacts"]["stdout"]["path"]
            .as_str()
            .unwrap();
        assert_eq!(
            first.artifacts.get(Path::new(pointer)),
            Some(&vec![255, 0, 1])
        );
        std::fs::create_dir(root.path().join("001.cases.jsonl.process-orphan")).unwrap();
        write(&receipt, b"second");
        let next = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        assert_eq!(next.records.len(), 2);
        assert_eq!(next.artifacts.len(), 6);
        assert_eq!(
            next.artifacts.get(Path::new(pointer)),
            Some(&vec![255, 0, 1])
        );
        assert!(root.path().join("001.cases.jsonl.process-orphan").is_dir());
    }
    #[test]
    fn a_sibling_receipt_cannot_reuse_another_shards_artifact_namespace() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("001.cases.jsonl");
        write(&receipt, b"first");
        let sibling = root.path().join("002.cases.jsonl");
        std::fs::copy(&receipt, &sibling).unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&sibling)
                .is_err()
        );
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_ok()
        );
    }
    #[test]
    fn changed_raw_bytes_and_extra_files_in_referenced_attempts_are_refused() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join("cases.jsonl");
        write(&receipt, b"first");
        let loaded = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = loaded.records[0]["process_observation"]["artifacts"]["stdout"]["path"]
            .as_str()
            .unwrap();
        let path = root.path().join(pointer);
        std::fs::write(&path, b"changed").unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_err()
        );
        std::fs::write(&path, b"first").unwrap();
        std::fs::write(path.with_file_name("unexpected.bin"), b"extra").unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_err()
        );
    }
    #[cfg(unix)]
    #[test]
    fn a_raw_artifact_symlink_cannot_escape_the_receipt_boundary() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let receipt = root.path().join("cases.jsonl");
        write(&receipt, b"first");
        let loaded = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = loaded.records[0]["process_observation"]["artifacts"]["stdout"]["path"]
            .as_str()
            .unwrap();
        let path = root.path().join(pointer);
        let target = outside.path().join("bytes");
        std::fs::write(&target, b"first").unwrap();
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(target, path).unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_err()
        );
    }
}
