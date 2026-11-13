//! The Linux profilers' names, arguments and output formats.
/// jemalloc's heap profile reader, from the system's jemalloc development package.
pub(super) const JEPROF: &str = "jeprof";
/// The Linux CPU profiler.
pub(super) const PERF: &str = "perf";
/// The system's symbol demangler, from binutils.
pub(super) const CXXFILT: &str = "c++filt";
/// The arguments that make `jeprof` print a dump's total.
pub(super) const HEAP_TOTAL: [&str; 2] = ["--text", "--show_bytes"];
/// The arguments that make `jeprof` print a dump's cumulative report with its source lines.
pub(super) const HEAP_REPORT: [&str; 5] = [
    "--text",
    "--cum",
    "--show_bytes",
    "--lines",
    "--nodecount=400",
];
/// The line of `jeprof`'s text report that holds the total, before the number.
pub(super) const TOTAL: &str = "Total:";
/// The extension of a heap dump.
pub(super) const DUMP_EXTENSION: &str = "heap";
/// `perf`'s recording command.
pub(super) const RECORD: &str = "record";
/// The `perf record` arguments before the rate: user-space CPU clock samples.
pub(super) const RECORD_EVENT: [&str; 3] = ["-e", "cpu-clock:u", "-F"];
/// The `perf record` arguments after the rate: DWARF call graphs with the largest user stack copy, written to the recording that follows.
pub(super) const RECORD_CALLS: [&str; 4] =
    ["--call-graph", "dwarf,65528", "--user-callchains", "-o"];
/// The separator before the recorded command.
pub(super) const COMMAND: &str = "--";
/// `perf`'s report command and the arguments every report shares, before the recording.
pub(super) const REPORT: [&str; 8] = [
    "report",
    "--no-inline",
    "--sort",
    "symbol",
    "--stdio",
    "-g",
    "none",
    "-i",
];
/// The self-time report: its `perf report` argument and its file.
pub(super) const SELF_REPORT: [&str; 2] = ["--no-children", "self.txt"];
/// The cumulative report: its `perf report` argument and its file.
pub(super) const CHILDREN_REPORT: [&str; 2] = ["--children", "children.txt"];
/// The receipt's file in the run's directory.
pub(super) const RECEIPT: &str = "receipt.json";
/// Refusal of a profiler that exits unsuccessfully.
pub(super) const TOOL_FAILED: &str = "profiler failed";
/// Refusal of a `jeprof` report without a total.
pub(super) const NO_TOTAL: &str = "jeprof printed no total";
/// Refusal of a `perf report` whose output could not be read.
pub(super) const NO_REPORT: &str = "perf report gave no output";
