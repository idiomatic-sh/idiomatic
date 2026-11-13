//! The receipt of one profile run, as written beside its reports.
use crate::domain::profiles::types::ProfileReceipt;
use crate::outbound::profile_tools::types::FileContents;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use serde::Serialize;
use std::path::PathBuf;

/// The receipt record.
#[derive(Serialize)]
struct ReceiptSchema {
    /// The directory the profiles were written to.
    directory: PathBuf,
    /// The largest heap dump.
    peak_dump: PathBuf,
    /// Its live heap, in bytes.
    peak_bytes: u64,
    /// The CPU profile's samples per second.
    sampling_rate: u32,
}

/// The receipt's JSON document.
pub(super) fn document(receipt: &ProfileReceipt) -> Result<FileContents, Error> {
    serde_json::to_vec_pretty(&ReceiptSchema {
        directory: receipt.directory.clone(),
        peak_dump: receipt.peak.dump.clone(),
        peak_bytes: receipt.peak.bytes.0,
        sampling_rate: receipt.rate.0,
    })
    .map(FileContents)
    .map_err(|error| Error::Json(Box::new(error)))
}
