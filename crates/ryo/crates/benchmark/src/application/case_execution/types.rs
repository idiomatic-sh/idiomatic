//! Data owned by the verification of one case attempt.
use crate::domain::process_failure::types::CaptureRecord;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;

/// One actual attempt, including operational failures without a fabricated report.
pub(crate) struct AttemptOutcome {
    /// The measurement, or why the attempt failed.
    pub result: Result<Measurement, ReportsError>,
    /// The raw process bytes of a failed decode.
    pub capture: CaptureRecord,
}
