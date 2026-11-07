//! The shape of the receipt an observation document carries: each wire shape is a `<Name>Schema` built from the core's type with an `impl From`; neutral data owns no serde implementation.
use ryo_benchmark_contracts::domain::context::types::ProjectContextReceipt;
use ryo_benchmark_contracts::domain::context::types::project_context_mode::value::ProjectContextMode;
use ryo_benchmark_contracts::domain::documents::types::text::value::CanonicalJson;
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
impl From<ProjectContextMode> for ModeSchema {
    fn from(mode: ProjectContextMode) -> Self {
        match mode {
            ProjectContextMode::Isolated => Self::Isolated,
            ProjectContextMode::ListedDisk => Self::ListedDisk,
        }
    }
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
impl<'a> From<&'a ProjectContextReceipt> for ReceiptSchema<'a> {
    fn from(receipt: &'a ProjectContextReceipt) -> Self {
        Self {
            mode: ModeSchema::from(receipt.mode),
            logical_root: &receipt.logical_root,
            actual_root: &receipt.actual_root,
            manifest_hash: &receipt.manifest_hash.0,
            dependencies: receipt
                .dependencies
                .iter()
                .map(|(path, state)| (path.as_path(), state.map(|fingerprint| fingerprint.0)))
                .collect(),
        }
    }
}

/// The canonical JSON text of one receipt.
pub(super) fn receipt_text(receipt: &ProjectContextReceipt) -> Result<CanonicalJson, Error> {
    serde_json::to_string(&ReceiptSchema::from(receipt))
        .map(CanonicalJson)
        .map_err(|error| Error::Invalid(error.to_string()))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
