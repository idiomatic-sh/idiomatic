//! The order of the heap dumps, the peak, the CPU sampling rate, the heap profiler's configuration and a `profile-ryo` command line.
use super::constants::{
    EXPECTED_PROFILE_ARGUMENTS, EXPECTED_PROFILE_DIRECTORY, FINAL_DUMP, HEAP_CONFIGURATION,
    HIGHEST_RATE, LOWEST_RATE, SAMPLES_PER_RECORDING,
};
use super::types::ProfileArguments;
use crate::domain::command_line::types::BenchmarkArgument;
use crate::outbound::profile_tools::types::{DumpTotal, SamplingRate};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::ffi::OsString;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The dumps in the order they were written: by the numbers in their names, compared as numbers, and the dump written at exit last.
pub(super) fn dump_order(mut dumps: Vec<PathBuf>) -> Vec<PathBuf> {
    dumps.sort_by_cached_key(|dump| {
        let name = dump
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let numbers = name
            .split(|character| !char::is_ascii_digit(&character))
            .filter(|digits| !digits.is_empty())
            .map(|digits| {
                let number = digits.trim_start_matches('0');
                (number.len(), number.to_owned())
            })
            .collect::<Vec<_>>();
        (name.contains(FINAL_DUMP), numbers)
    });
    dumps
}

/// The largest dump; the first in order on a tie, none when there is no dump.
pub(super) fn peak(totals: &[DumpTotal]) -> Option<&DumpTotal> {
    totals.iter().fold(None, |highest, total| match highest {
        Some(highest) if highest.bytes.0 >= total.bytes.0 => Some(highest),
        _ => Some(total),
    })
}

/// The CPU profiler's rate: about 60 000 samples over the run's wall time and threads, between 1 and 999 a second.
pub(super) fn sampling_rate(wall: Duration, threads: NonZeroUsize) -> SamplingRate {
    let spread = wall
        .as_millis()
        .saturating_mul(u128::try_from(threads.get()).unwrap_or(u128::MAX));
    let rate = SAMPLES_PER_RECORDING
        .checked_div(spread)
        .unwrap_or(HIGHEST_RATE)
        .clamp(LOWEST_RATE, HIGHEST_RATE);
    SamplingRate(u32::try_from(rate).unwrap_or(u32::MAX))
}

/// The heap profiler's configuration, writing its dumps under `prefix`.
pub(super) fn heap_configuration(prefix: &Path) -> OsString {
    let mut configuration = OsString::from(HEAP_CONFIGURATION);
    configuration.push(prefix);
    configuration
}

/// The worker's arguments and the directory of a `profile-ryo` command line, given the arguments after its command word.
///
/// # Errors
/// Refuses an empty argument list and one without a directory after the worker request.
pub(super) fn split_profile(
    mut arguments: Vec<BenchmarkArgument>,
) -> Result<ProfileArguments, ReportsError> {
    let directory = arguments
        .pop()
        .ok_or_else(|| ReportsError::Invalid(EXPECTED_PROFILE_ARGUMENTS.into()))?;
    if arguments.is_empty() {
        return Err(ReportsError::Invalid(EXPECTED_PROFILE_DIRECTORY.into()));
    }
    Ok(ProfileArguments {
        worker: arguments,
        directory: PathBuf::from(directory.0),
    })
}
