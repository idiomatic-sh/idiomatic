//! Filesystem encoding and atomic publication of complete inventory snapshots.
use super::constants;
use super::types::{AuditJson, InvocationBytes, RegistrationBytes, RowJson, SummaryJson};
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::inventory::types::{
    ContextFiles, RawArtifacts, ReceiptFile, Registration, Row, Summary,
};
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::ObservationDocument;
use ryo_benchmark_contracts::domain::documents::types::reference_discrepancy::value::ArtifactPointer;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;
use ryo_benchmark_contracts::domain::receipts::constants::{
    CASES_JSONL, INVOCATION_JSON, STDERR_BIN, STDOUT_BIN,
};
use ryo_benchmark_contracts::domain::receipts::types::ReceiptAudit;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write as _,
    path::{Component, Path, PathBuf},
};

/// Publish a whole snapshot atomically, retaining the previous valid snapshot on failure.
fn publish(output: &Path, summary: &Summary) -> Result<(), Error> {
    std::fs::create_dir_all(output)?;
    let mut temporary = tempfile::NamedTempFile::new_in(output)?;
    temporary.write_all(&SummaryJson::try_from(summary)?.0.0)?;
    temporary.as_file().sync_all()?;
    let _persisted = temporary
        .persist(output.join(constants::SUMMARY_JSON))
        .map_err(|error| error.error)?;
    Ok(())
}

/// Refuse stale result reuse while allowing the provider's already staged reproductions.
fn begin(output: &Path) -> Result<(), Error> {
    if std::fs::metadata(output.join(constants::SUMMARY_JSON)).is_ok() {
        return Err(Error::Invalid(
            constants::INVENTORY_OUTPUT_ALREADY_CONTAINS_A_SUMMARY.into(),
        ));
    }
    Ok(std::fs::create_dir_all(output)?)
}

/// Each invocation owns fresh native case receipts.
fn prepare(observations: &Path) -> Result<(), Error> {
    if [CASES_JSONL, STDOUT_BIN, STDERR_BIN, constants::RESULT_JSON]
        .iter()
        .any(|name| std::fs::metadata(observations.join(name)).is_ok())
    {
        return Err(Error::Invalid(
            constants::INVENTORY_RULE_OUTPUT_ALREADY_CONTAINS_RECEI.into(),
        ));
    }
    Ok(std::fs::create_dir_all(observations)?)
}

/// Keep the exact child streams before interpreting its result.
fn capture(
    observations: &Path,
    stdout: &CapturedBytes,
    stderr: &CapturedBytes,
) -> Result<ReceiptFile, Error> {
    std::fs::create_dir_all(observations)?;
    std::fs::write(observations.join(STDOUT_BIN), &stdout.0)?;
    std::fs::write(observations.join(STDERR_BIN), &stderr.0)?;
    let receipt = observations.join(CASES_JSONL);
    Ok(if receipt.is_file() {
        ReceiptFile::Present(receipt)
    } else {
        ReceiptFile::Absent
    })
}

/// Persist one terminal row beside its raw receipts.
fn result(observations: &Path, row: &Row) -> Result<(), Error> {
    std::fs::create_dir_all(observations)?;
    Ok(std::fs::write(
        observations.join(constants::RESULT_JSON),
        RowJson::try_from(row)?.0.0,
    )?)
}

/// List the regular files of a canonical registered context.
fn context_files(context: &Path) -> Result<ContextFiles, Error> {
    let root = context.canonicalize()?;
    if root != context {
        return Err(Error::Invalid(
            constants::REGISTERED_CONTEXT_MUST_BE_CANONICAL_AND.into(),
        ));
    }
    let mut pending = vec![root];
    let mut files = BTreeSet::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                let _new = files.insert(entry.path());
            } else {
                return Err(Error::Invalid(
                    constants::REGISTERED_CONTEXT_CONTAINS_NON_REGULAR_ENTR.into(),
                ));
            }
        }
    }
    Ok(ContextFiles { files })
}

/// Read only referenced receipt-file attempts; retain unreferenced resume history untouched.
fn raw_artifacts(receipt: &Path, rows: &[ObservationDocument]) -> Result<RawArtifacts, Error> {
    let name = receipt
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::Invalid(constants::RECEIPT_FILENAME_IS_NOT_UTF8.into()))?;
    let parent = receipt
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new(Component::CurDir.as_os_str()))
        .canonicalize()?;
    let prefix = format!("{name}.process-");
    let mut bytes = BTreeMap::new();
    let mut invocations = BTreeMap::new();
    let mut directories = BTreeSet::new();
    for row in rows {
        let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ReferenceProcess::Observed(process) = &row.process_observation else {
            continue;
        };
        let ryo_benchmark_contracts::domain::documents::types::observation_document::value::ProcessArtifactsRecord::Attached(artifacts) = &process.artifacts else {
            continue;
        };
        for (raw, filename) in [
            (&artifacts.stdout, STDOUT_BIN),
            (&artifacts.stderr, STDERR_BIN),
            (&artifacts.invocation, INVOCATION_JSON),
        ] {
            let ArtifactPointer { path: text, sha256 } = raw;
            let text = text.0.as_str();
            let path = PathBuf::from(text);
            let components = path.components().collect::<Vec<_>>();
            if text.contains(['\\', ':'])
                || components.len() != 3
                || !components
                    .iter()
                    .all(|part| matches!(part, Component::Normal(_)))
            {
                return Err(Error::Invalid(
                    constants::UNSAFE_RAW_PROCESS_ARTIFACT_PATH.into(),
                ));
            }
            let attempt = components[0]
                .as_os_str()
                .to_str()
                .ok_or_else(|| Error::Invalid(constants::NONUTF8_ATTEMPT.into()))?;
            let key = components[1]
                .as_os_str()
                .to_str()
                .ok_or_else(|| Error::Invalid(constants::NONUTF8_CASE_KEY.into()))?;
            if !attempt.starts_with(&prefix)
                || attempt.len() == prefix.len()
                || key.len() != constants::DIGEST_HEX_LENGTH
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                || components[2].as_os_str() != filename
            {
                return Err(Error::Invalid(
                    constants::RAW_ARTIFACT_IS_OUTSIDE_ITS_RECEIPT.into(),
                ));
            }
            let full = parent.join(&path).canonicalize()?;
            if !full.starts_with(&parent) || !full.is_file() {
                return Err(Error::Invalid(
                    constants::RAW_ARTIFACT_ESCAPES_ITS_RECEIPT_BOUNDARY.into(),
                ));
            }
            let data = std::fs::read(&full)?;
            if sha256.0 != format!("{:x}", Sha256::digest(&data)) {
                return Err(Error::Invalid(
                    constants::RAW_PROCESS_ARTIFACT_IS_STALE_OR.into(),
                ));
            }
            if filename == INVOCATION_JSON {
                let _previous = invocations.insert(
                    path.clone(),
                    InvocationDocument::try_from(InvocationBytes(&CapturedBytes(data.clone())))?,
                );
            }
            if bytes.insert(path.clone(), CapturedBytes(data)).is_some() {
                return Err(Error::Invalid(
                    constants::RAW_PROCESS_ARTIFACT_IS_STALE_OR.into(),
                ));
            }
            let _new = directories.insert(
                full.parent()
                    .ok_or_else(|| {
                        Error::Invalid(constants::RAW_ARTIFACT_HAS_NO_CASE_DIRECTORY.into())
                    })?
                    .to_path_buf(),
            );
        }
    }
    for directory in directories {
        let mut names = std::fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        if names != [INVOCATION_JSON, STDERR_BIN, STDOUT_BIN].map(std::ffi::OsString::from) {
            return Err(Error::Invalid(
                constants::EXTRA_RAW_ARTIFACTS_IN_REFERENCED_CASE.into(),
            ));
        }
    }
    Ok(RawArtifacts { bytes, invocations })
}

/// Local registrations and atomic verification artifacts.
#[derive(Debug)]
pub(crate) struct InventoryLocalAdapter;

impl InventoryPort for InventoryLocalAdapter {
    fn load(&self, path: &Path) -> Result<Registration, Error> {
        Registration::try_from(RegistrationBytes(&CapturedBytes(std::fs::read(path)?)))
    }
    fn begin(&self, output: &Path) -> Result<(), Error> {
        begin(output)
    }
    fn publish(&self, output: &Path, summary: &Summary) -> Result<(), Error> {
        publish(output, summary)
    }
    fn prepare(&self, observations: &Path) -> Result<(), Error> {
        prepare(observations)
    }
    fn capture(
        &self,
        observations: &Path,
        stdout: &CapturedBytes,
        stderr: &CapturedBytes,
    ) -> Result<ReceiptFile, Error> {
        capture(observations, stdout, stderr)
    }
    fn result(&self, observations: &Path, row: &Row) -> Result<(), Error> {
        result(observations, row)
    }
    fn decode_invocation(&self, bytes: &CapturedBytes) -> Result<InvocationDocument, Error> {
        InvocationDocument::try_from(InvocationBytes(bytes))
    }
    fn raw_artifacts(
        &self,
        receipt: &Path,
        records: &[ObservationDocument],
    ) -> Result<RawArtifacts, Error> {
        raw_artifacts(receipt, records)
    }
    fn context_files(&self, context: &Path) -> Result<ContextFiles, Error> {
        context_files(context)
    }
    fn audit_report(&self, audit: &ReceiptAudit) -> Result<CapturedBytes, Error> {
        AuditJson::try_from(audit).map(|json| json.0)
    }
}
