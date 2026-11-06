//! Receipt-file scoped process artifacts; resumes never overwrite historical attempts.
use super::observation::schemas::ObservationSchema;
use crate::outbound::suite::adapters::local::constants;
use crate::outbound::suite::types::{
    discrepancy::ArtifactError,
    observation_document::{ObservationDocument, ProcessArtifacts},
    process::CapturedProcess,
    reference_discrepancy::ArtifactPointer,
    text::{PathText, Sha256Text},
};
use sha2::{Digest as _, Sha256};
use std::{io::Write as _, path::Path};

/// Persist one exact observation, adding storage-only raw artifact paths/digests.
pub(super) fn record(
    receipt: &Path,
    mut record: ObservationDocument,
    process: Option<CapturedProcess<'_>>,
) -> Result<(), ArtifactError> {
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| ArtifactError::Invalid(constants::RECEIPT_FILENAME_MUST_BE_UTF8.into()))?;
    let parent = receipt
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new(constants::DOT));
    if name.contains(['\\', ':'])
        || std::fs::symlink_metadata(receipt).is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(ArtifactError::Invalid(
            constants::UNSAFE_RECEIPT_FILE_IDENTITY.into(),
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
                constants::INVALID_FRAMED_PROCESS_CASE_KEY.into(),
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
        let mut artifacts = std::collections::BTreeMap::new();
        for (label, filename, bytes) in [
            (constants::STDOUT, constants::STDOUT_BIN, process.stdout),
            (constants::STDERR, constants::STDERR_BIN, process.stderr),
            (
                constants::INVOCATION,
                constants::INVOCATION_JSON,
                invocation.as_slice(),
            ),
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
                        ArtifactError::Invalid(constants::RAW_ARTIFACT_PATH_IS_NOT_UTF8.into())
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(constants::SLASH);
            artifacts.insert(
                label,
                ArtifactPointer {
                    path: PathText(relative),
                    sha256: Sha256Text(format!("{:x}", Sha256::digest(bytes))),
                },
            );
        }
        let (Some(stdout), Some(stderr), Some(invocation), Some(observation)) = (
            artifacts.remove(constants::STDOUT),
            artifacts.remove(constants::STDERR),
            artifacts.remove(constants::INVOCATION),
            record.process_observation.as_mut(),
        ) else {
            return Err(ArtifactError::Invalid(
                constants::PROCESS_OBSERVATION_NEEDS_RAW_ARTIFACTS.into(),
            ));
        };
        observation.artifacts = Some(ProcessArtifacts {
            stdout,
            stderr,
            invocation,
        });
    }
    let mut bytes = serde_json::to_vec(&ObservationSchema::from(&record))
        .map_err(|error| ArtifactError::Invalid(error.to_string()))?;
    bytes.push(b'\n');
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(receipt)?;
    output.write_all(&bytes)?;
    output.sync_data()?;
    Ok(())
}

/// Read every row of one receipt file; absence and malformed or unknown fields remain errors.
pub(super) fn read(receipt: &Path) -> Result<Vec<ObservationDocument>, ArtifactError> {
    std::fs::read_to_string(receipt)?
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str::<ObservationSchema>(line)
                .map(ObservationDocument::from)
                .map_err(|error| {
                    ArtifactError::Invalid(format!("receipt row {}: {error}", index + 1))
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outbound::{
        inventory::interface::InventoryPort as _,
        suite::{interface::SuitePort as _, types::ArtifactPath},
    };
    fn row() -> ObservationDocument {
        use crate::outbound::suite::types::{
            observation_document::{
                ContextRun, ExitStatus, FileMetadataDigest, IntentMatch, ProcessObservation,
                ProjectContextRecord, ProjectInputsDigest, ReferenceExpectation,
                RuleCohort, RuleContextRecord, RuleObservationsRecord, SourceIdentity,
                ToolContractRecord,
            },
            reference_discrepancy::ExpectationKind,
            text::{CaseName, ReasonText, RuleName, SchemaName, Side, YesNo},
        };
        use ryo_benchmark_contracts::domain::reports::types::ComparisonContract;
        ObservationDocument {
            rule: RuleName(constants::CHECK001.into()),
            name: CaseName(constants::ONE.into()),
            side: Side::Reference,
            complete: YesNo::Yes,
            error: None::<ReasonText>,
            case_passed: YesNo::Yes,
            files: Vec::new(),
            source_sha256: SourceIdentity::Unavailable,
            file_metadata_sha256: FileMetadataDigest::Undeclared,
            project_inputs_sha256: ProjectInputsDigest::Undeclared,
            expected: Vec::new(),
            reference_expected: ReferenceExpectation::Underivable,
            diagnostics: None,
            skipped: None,
            observation_schema: SchemaName("benchmark-case/v2".into()),
            expectation_contract: ComparisonContract::Diagnostics,
            execution_metadata: None,
            coverage: None,
            context_manifest: ContextRun::Isolated,
            project_context: ProjectContextRecord::NotCarried,
            rule_context: RuleContextRecord::Undeclared,
            rule_observations: RuleObservationsRecord::Undeclared,
            selected_rules: RuleCohort::Undeclared,
            actual_request_root: None,
            documented_reference_discrepancy: YesNo::No,
            documented_tool_contract_difference: ToolContractRecord::Unrecorded,
            discrepancy_validation: None,
            raw_parity_matches_intended: None,
            diagnostics_match_intended: IntentMatch::Unreported,
            process_observation: Some(ProcessObservation {
                kind: ExpectationKind::ProcessFailure,
                exit: ExitStatus(2),
                signal: None,
                actual_request_root: PathText("/cases/root".into()),
                observer_no_deadline: YesNo::Yes,
                executable_sha256: None,
                settings_sha256: None,
                metadata_error: None,
                template_sha256: None,
                artifacts: None,
            }),
        }
    }
    fn write(receipt: &Path, stdout: &[u8]) {
        crate::outbound::suite::factory::local()
            .record_case(
                ArtifactPath(receipt),
                row(),
                Some(CapturedProcess {
                    key: &constants::A.repeat(64),
                    invocation: &serde_json::json!({constants::EXIT:2}),
                    stdout,
                    stderr: constants::ERROR,
                }),
            )
            .unwrap();
    }
    #[test]
    fn raw_bytes_attempts_and_unreferenced_resume_history_are_retained() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join(constants::N_001_CASES_JSONL);
        write(&receipt, &[255, 0, 1]);
        let first = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = first.records[0]
            .process_observation
            .as_ref()
            .and_then(|process| process.artifacts.as_ref())
            .unwrap()
            .stdout
            .path
            .0
            .as_str();
        assert_eq!(
            first.artifacts.get(Path::new(pointer)),
            Some(&vec![255, 0, 1])
        );
        std::fs::create_dir(
            root.path()
                .join(constants::N_001_CASES_JSONL_PROCESS_ORPHAN),
        )
        .unwrap();
        write(&receipt, constants::SECOND);
        let next = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        assert_eq!(next.records.len(), 2);
        assert_eq!(next.artifacts.len(), 6);
        assert_eq!(
            next.artifacts.get(Path::new(pointer)),
            Some(&vec![255, 0, 1])
        );
        assert!(
            root.path()
                .join(constants::N_001_CASES_JSONL_PROCESS_ORPHAN)
                .is_dir()
        );
    }
    #[test]
    fn a_sibling_receipt_cannot_reuse_another_shards_artifact_namespace() {
        let root = tempfile::tempdir().unwrap();
        let receipt = root.path().join(constants::N_001_CASES_JSONL);
        write(&receipt, constants::FIRST);
        let sibling = root.path().join(constants::N_002_CASES_JSONL);
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
        let receipt = root.path().join(constants::CASES_JSONL);
        write(&receipt, constants::FIRST);
        let loaded = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = loaded.records[0]
            .process_observation
            .as_ref()
            .and_then(|process| process.artifacts.as_ref())
            .unwrap()
            .stdout
            .path
            .0
            .as_str();
        let path = root.path().join(pointer);
        std::fs::write(&path, constants::CHANGED).unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_err()
        );
        std::fs::write(&path, constants::FIRST).unwrap();
        std::fs::write(
            path.with_file_name(constants::UNEXPECTED_BIN),
            constants::EXTRA,
        )
        .unwrap();
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
        let receipt = root.path().join(constants::CASES_JSONL);
        write(&receipt, constants::FIRST);
        let loaded = crate::outbound::inventory::factory::local()
            .observations(&receipt)
            .unwrap();
        let pointer = loaded.records[0]
            .process_observation
            .as_ref()
            .and_then(|process| process.artifacts.as_ref())
            .unwrap()
            .stdout
            .path
            .0
            .as_str();
        let path = root.path().join(pointer);
        let target = outside.path().join(constants::BYTES);
        std::fs::write(&target, constants::FIRST).unwrap();
        std::fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(target, path).unwrap();
        assert!(
            crate::outbound::inventory::factory::local()
                .observations(&receipt)
                .is_err()
        );
    }
}
