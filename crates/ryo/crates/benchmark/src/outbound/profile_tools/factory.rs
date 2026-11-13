//! Builds the profile tools adapters.
use super::adapters::linux::adapter::ProfileToolsLinuxAdapter;
/// Construct the Linux profilers: jemalloc's `jeprof` for the heap, `perf` for the CPU, `c++filt` to demangle.
#[must_use]
pub fn linux() -> impl super::interface::ProfileToolsPort {
    ProfileToolsLinuxAdapter
}
