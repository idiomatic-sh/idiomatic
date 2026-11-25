//! What the `process_meter` port asks of the operating system about this process.
use super::types::CpuTime;
use benchmark_contracts::domain::measurements::types::ResidentBytes;
use benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Reads this process's CPU time and peak resident memory (Linux: the process clock, `/proc/self/status` VmHWM and
/// `/proc/self/clear_refs`).
pub trait ProcessMeterPort {
    /// The CPU time this process has used so far.
    ///
    /// # Errors
    /// The clock cannot be read.
    fn cpu(&self) -> Result<CpuTime, Error>;
    /// Reset the peak resident memory to the current resident memory.
    ///
    /// # Errors
    /// The reset is refused.
    fn reset_peak(&self) -> Result<(), Error>;
    /// The peak resident memory since the last reset.
    ///
    /// # Errors
    /// The peak cannot be read.
    fn peak(&self) -> Result<ResidentBytes, Error>;
}
