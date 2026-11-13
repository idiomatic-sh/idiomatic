//! The order of the heap dumps, the peak, the CPU sampling rate, the heap profiler's configuration and a `profile-ryo` command line.
use super::types::{DumpTotal, ProfileArguments, SamplingRate};
use crate::domain::command_line::types::BenchmarkArgument;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The dumps in the order they were written: by the numbers in their names, compared as numbers, and the dump written at exit last.
pub(crate) fn dump_order(dumps: Vec<PathBuf>) -> Vec<PathBuf> {
    todo!("{dumps:?}")
}

/// The largest dump; the first in order on a tie, none when there is no dump.
pub(crate) fn peak(totals: &[DumpTotal]) -> Option<&DumpTotal> {
    todo!("{totals:?}")
}

/// The CPU profiler's rate: about 60 000 samples over the run's wall time and threads, between 1 and 999 a second.
pub(crate) fn sampling_rate(wall: Duration, threads: NonZeroUsize) -> SamplingRate {
    todo!("{wall:?} {threads}")
}

/// The heap profiler's configuration, writing its dumps under `prefix`.
pub(crate) fn heap_configuration(prefix: &Path) -> OsString {
    todo!("{prefix:?}")
}

/// The worker's arguments and the directory of a `profile-ryo` command line, given the arguments after its command word.
///
/// # Errors
/// Refuses an empty argument list and one without a directory after the worker request.
pub(crate) fn split_profile(
    arguments: Vec<BenchmarkArgument>,
) -> Result<ProfileArguments, ReportsError> {
    todo!("{arguments:?}")
}
