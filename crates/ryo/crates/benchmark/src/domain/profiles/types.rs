//! The values a profile run records.
use std::path::PathBuf;

/// The live heap of one dump, in bytes, as the heap profiler totals it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeapBytes(pub u64);

/// The CPU profiler's samples per second.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplingRate(pub u32);

/// One heap dump and its total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DumpTotal {
    /// The dump file.
    pub dump: PathBuf,
    /// Its live heap.
    pub bytes: HeapBytes,
}

/// What one profile run recorded, written beside its reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileReceipt {
    /// The directory the profiles were written to.
    pub directory: PathBuf,
    /// The largest heap dump.
    pub peak: DumpTotal,
    /// The rate the CPU profile sampled at.
    pub rate: SamplingRate,
}

/// The arguments of a `profile-ryo` command line, split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileArguments {
    /// The worker's own arguments: its request and any configuration.
    pub(crate) worker: Vec<crate::domain::command_line::types::BenchmarkArgument>,
    /// The directory the profiles are written to.
    pub(crate) directory: PathBuf,
}
