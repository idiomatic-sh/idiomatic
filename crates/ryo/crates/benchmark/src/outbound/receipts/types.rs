//! The types of the `receipts` port: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::measurements::types::CapturedBytes;

/// The receipt file one operation appends to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiptPath<'a>(pub &'a std::path::Path);

/// Where a run appends its raw case receipts, as the edge chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CaseReceipts {
    /// The run records no case receipts.
    Unrecorded,
    /// The run appends its case receipts to this file.
    Recorded(std::path::PathBuf),
}

/// Captured verification-only bytes to persist beside one receipt file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedProcess<'a> {
    /// Collision-safe key framed from rule/case/side by the application.
    pub key: &'a ArtifactDigest,
    /// Actual root/files/normalized settings/executable/status provenance.
    pub invocation: &'a InvocationDocument,
    /// Exact unmodified stdout.
    pub stdout: &'a CapturedBytes,
    /// Exact unmodified stderr.
    pub stderr: &'a CapturedBytes,
}
