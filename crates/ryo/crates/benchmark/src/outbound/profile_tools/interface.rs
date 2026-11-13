//! The heap and CPU profilers, their reports and the files of one profile run.
use super::types::{FileContents, HeapBytes, HeapDumps, ProfileReceipt, SamplingRate};
use ryo_benchmark_contracts::domain::measurements::types::Invocation;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use std::num::NonZeroUsize;
use std::path::Path;

/// The profilers a profile run uses; the worker itself runs through the execution port.
pub trait ProfileToolsPort {
    /// The threads the worker spreads its work over.
    fn threads(&self) -> NonZeroUsize;
    /// Make the folder `output`, where a run's files go, and its parents.
    ///
    /// # Errors
    /// Fails when the folder cannot be made.
    fn prepare(&self, output: &Path) -> Result<(), Error>;
    /// The heap dumps written into the folder `dumps`, in no particular order.
    ///
    /// # Errors
    /// Fails when the directory cannot be read.
    fn dumps(&self, dumps: &Path) -> Result<HeapDumps, Error>;
    /// Write `contents` to `file`.
    ///
    /// # Errors
    /// Fails when the file cannot be written.
    fn write(&self, file: &Path, contents: &FileContents) -> Result<(), Error>;
    /// The live heap of one dump of `worker`.
    ///
    /// # Errors
    /// Fails when the heap profiler fails or prints no total.
    fn heap_total(&self, worker: &Path, dump: &Path) -> Result<HeapBytes, Error>;
    /// Write the report of one dump of `worker`, with its source lines, to `report`.
    ///
    /// # Errors
    /// Fails when the heap profiler fails or the report cannot be written.
    fn heap_report(&self, worker: &Path, dump: &Path, report: &Path) -> Result<(), Error>;
    /// The `worker` invocation under the CPU profiler, sampling at `rate` into `recording`.
    fn cpu_invocation(
        &self,
        worker: Invocation,
        rate: SamplingRate,
        recording: &Path,
    ) -> Invocation;
    /// Write the self and cumulative reports of `recording` into the folder `reports`.
    ///
    /// # Errors
    /// Fails when the CPU profiler or the demangler fails, or a report cannot be written.
    fn cpu_report(&self, recording: &Path, reports: &Path) -> Result<(), Error>;
    /// Write the run's receipt into its folder `output`.
    ///
    /// # Errors
    /// Fails when the receipt cannot be encoded or written.
    fn receipt(&self, output: &Path, receipt: &ProfileReceipt) -> Result<(), Error>;
}
