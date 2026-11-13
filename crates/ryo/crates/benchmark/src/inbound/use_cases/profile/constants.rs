//! The layout of a profile run's directory, and its refusals.
/// Cargo's harness argument, which a bench's command line drops.
pub(super) const CARGO_BENCH_ARGUMENT: &str = "--bench";
/// The heap profile's directory.
pub(super) const HEAP: &str = "heap";
/// The heap dumps' directory, in the heap profile's.
pub(super) const DUMPS: &str = "dumps";
/// The name every heap dump starts with.
pub(super) const DUMP_PREFIX: &str = "jeprof";
/// The CPU profile's directory.
pub(super) const CPU: &str = "cpu";
/// The CPU recording, in the CPU profile's directory.
pub(super) const RECORDING: &str = "perf.data";
/// A child's standard output, in its profile's directory.
pub(super) const STDOUT: &str = "stdout.bin";
/// Every dump's total, in the heap profile's directory.
pub(super) const TOTALS: &str = "totals.txt";
/// The peak dump's report, in the heap profile's directory.
pub(super) const PEAK: &str = "peak.txt";
/// The span trace the bench writes after the profiles.
pub(super) const TRACE: &str = "trace.json";
/// The environment variable the worker's jemalloc reads its configuration from.
pub(super) const HEAP_PROFILER_CONFIGURATION: &str = "_RJEM_MALLOC_CONF";
/// Refusal of a run whose worker wrote no heap dump.
pub(super) const NO_HEAP_DUMP: &str =
    "no heap dump: build with `--features idiomatic-sh-ryo-benchmark/heap-profile`";
/// Refusal of a run whose two children answered differently.
pub(super) const DIFFERING_STDOUT: &str =
    "the heap-profiled and the CPU-profiled worker printed different output";
/// Refusal of a run whose child failed.
pub(super) const CHILD_FAILED: &str = "the profiled worker failed";
