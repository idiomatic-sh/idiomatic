//! Data owned by the verification of one case attempt.
use ryo_benchmark_contracts::domain::documents::types::invocation_document::value::InvocationDocument;
use ryo_benchmark_contracts::domain::measurements::types::{Measurement, ProcessOutput};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// Raw process bytes exist independently of whether a report could be decoded.
pub struct ProcessCapture {
    /// The raw invocation record kept beside the process bytes.
    pub invocation: InvocationDocument,
    /// The exact process output.
    pub output: ProcessOutput,
}

/// Whether an attempt kept its raw process bytes.
pub enum CaptureRecord {
    /// The report decoded, so no raw bytes were kept.
    Absent,
    /// The report did not decode, so the raw bytes were kept.
    Kept(ProcessCapture),
}

/// One actual attempt, including operational failures without a fabricated report.
pub struct AttemptOutcome {
    /// The measurement, or why the attempt failed.
    pub result: Result<Measurement, ReportsError>,
    /// The raw process bytes of a failed decode.
    pub capture: CaptureRecord,
}
