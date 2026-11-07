//! The adapter's traversal and serialization of the receipt file and the raw process artifacts beside it.
use super::constants;
use crate::outbound::receipts::errors::ReceiptsError;
use crate::outbound::receipts::types::CapturedProcess;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ObservationDocument, ProcessArtifacts};
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ArtifactPointer;
use ryo_benchmark_contracts::domain::documents::types::text::value::{PathText, Sha256Text};
use sha2::{Digest as _, Sha256};
use std::{io::Write as _, path::Path};

/// Persist one exact observation, adding storage-only raw artifact paths/digests.
pub(super) fn record(
    receipt: &Path,
    mut record: ObservationDocument,
    process: Option<CapturedProcess<'_>>,
) -> Result<(), ReceiptsError> {
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| ReceiptsError::Invalid(constants::RECEIPT_FILENAME_MUST_BE_UTF8.into()))?;
    let parent = receipt
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new(constants::DOT));
    if name.contains(['\\', ':'])
        || std::fs::symlink_metadata(receipt).is_ok_and(|meta| meta.file_type().is_symlink())
    {
        return Err(ReceiptsError::Invalid(
            constants::UNSAFE_RECEIPT_FILE_IDENTITY.into(),
        ));
    }
    std::fs::create_dir_all(parent)?;
    let boundary = parent.canonicalize()?;
    if let Some(process) = process {
        if process.key.0.len() != 64
            || !process
                .key
                .0
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(ReceiptsError::Invalid(
                constants::INVALID_FRAMED_PROCESS_CASE_KEY.into(),
            ));
        }
        // Kept directories survive incomplete receipts; only referenced attempts are loaded later.
        let directory = tempfile::Builder::new()
            .prefix(&format!("{name}.process-"))
            .tempdir_in(&boundary)?
            .keep();
        let case = directory.join(&process.key.0);
        std::fs::create_dir(&case)?;
        let invocation = super::invocation::schemas::bytes(process.invocation)?;
        let mut artifacts = std::collections::BTreeMap::new();
        for (label, filename, bytes) in [
            (constants::STDOUT, constants::STDOUT_BIN, process.stdout.0.as_slice()),
            (constants::STDERR, constants::STDERR_BIN, process.stderr.0.as_slice()),
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
                .map_err(|error| ReceiptsError::Invalid(error.to_string()))?;
            let relative = relative
                .components()
                .map(|part| {
                    part.as_os_str().to_str().ok_or_else(|| {
                        ReceiptsError::Invalid(constants::RAW_ARTIFACT_PATH_IS_NOT_UTF8.into())
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
            match &mut record.process_observation {
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(process) => Some(process),
                ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::NotObserved => None,
            },
        ) else {
            return Err(ReceiptsError::Invalid(
                constants::PROCESS_OBSERVATION_NEEDS_RAW_ARTIFACTS.into(),
            ));
        };
        observation.artifacts = ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(ProcessArtifacts {
            stdout,
            stderr,
            invocation,
        });
    }
    let mut bytes = super::observation::schemas::row_bytes(&record)?;
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
pub(super) fn read(receipt: &Path) -> Result<Vec<ObservationDocument>, ReceiptsError> {
    super::rows::schemas::rows(&std::fs::read_to_string(receipt)?)
}
