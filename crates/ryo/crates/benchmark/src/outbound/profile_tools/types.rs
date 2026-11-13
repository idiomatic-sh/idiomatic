//! Plain data of the profile tools port: what the profilers answer and record.
use std::path::PathBuf;

/// The heap dump files one run wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeapDumps(pub Vec<PathBuf>);

/// The bytes one file of a profile run holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileContents(pub Vec<u8>);

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
