//! Data of the raw process an attempt keeps.
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::measurements::types::ProcessOutput;

/// Raw process bytes exist independently of whether a report could be decoded.
pub(crate) struct ProcessCapture {
    /// The raw invocation record kept beside the process bytes.
    pub invocation: InvocationDocument,
    /// The exact process output.
    pub output: ProcessOutput,
}

/// Whether an attempt kept its raw process bytes.
pub(crate) enum CaptureRecord {
    /// The report decoded, so no raw bytes were kept.
    Absent,
    /// The report did not decode, so the raw bytes were kept.
    Kept(ProcessCapture),
}
