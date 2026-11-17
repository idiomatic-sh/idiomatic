//! The one CPU floor and memory bound every language's growth is held to: Ruff's.
use super::types::{CpuFloor, MemoryBound};
use ryo_benchmark_contracts::domain::measurements::types::{DoublingRatio, ResidentBytes};

/// A megabyte as the memory bound counts it: the kibibytes `/proc` reports, a thousand and twenty-four of them.
pub const MEGABYTE: u64 = 1_024 * 1_024;
/// The CPU time under which a half-size time counts as this much (Ruff's 0.3 s), so a fixed startup cost does not decide a
/// ratio.
pub const CPU_FLOOR: CpuFloor = CpuFloor(std::time::Duration::from_millis(300));
/// How much memory a grown program may hold (Ruff's): 160 MB before its program counts, 100 MB per thousand of its whole
/// size, and at most 2.4 times the half-size peak, a half-size peak under 120 MB counting as 120 MB.
pub const MEMORY_BOUND: MemoryBound = MemoryBound {
    base: ResidentBytes(160 * MEGABYTE),
    per_thousand: ResidentBytes(100 * MEGABYTE),
    doubling: DoublingRatio(2.4),
    floor: ResidentBytes(120 * MEGABYTE),
};
