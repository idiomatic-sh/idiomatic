//! The receipt as an observation document carries it; neutral data owns no serde implementation.
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Explicit project lookup mode of a carried receipt.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ModeSchema {
    Isolated,
    ListedDisk,
}

/// The actual dependency observations a carried receipt records.
#[derive(Debug, Serialize)]
struct ReceiptSchema<'a> {
    mode: ModeSchema,
    logical_root: &'a Path,
    actual_root: &'a Path,
    manifest_hash: &'a str,
    dependencies: BTreeMap<&'a Path, Option<[u8; 16]>>,
}

/// The canonical JSON text of one receipt.
pub(super) fn receipt_text(receipt: &ProjectContextReceipt) -> Result<String, Error> {
    serde_json::to_string(&ReceiptSchema {
        mode: match receipt.mode {
            ProjectContextMode::Isolated => ModeSchema::Isolated,
            ProjectContextMode::ListedDisk => ModeSchema::ListedDisk,
        },
        logical_root: &receipt.logical_root,
        actual_root: &receipt.actual_root,
        manifest_hash: &receipt.manifest_hash.0,
        dependencies: receipt
            .dependencies
            .iter()
            .map(|(path, state)| (path.as_path(), state.map(|fingerprint| fingerprint.0)))
            .collect(),
    })
    .map_err(|error| Error::Invalid(error.to_string()))
}
