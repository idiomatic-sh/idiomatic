//! Evidence stream protocol.
/// Refusal when the injected evidence writer lock cannot be recovered.
pub(super) const POISONED: &str = "evidence output lock poisoned";
/// One complete BMF document is emitted per output line.
pub(super) const NEWLINE: &[u8] = b"\n";

/// Largest integer byte count that the BMF floating metric can represent exactly.
pub(super) const MAX_EXACT_RESIDENT: u64 = 1_u64 << f64::MANTISSA_DIGITS;
/// Refuse evidence that would silently round a resident-byte observation.
pub(super) const INEXACT_RESIDENT: &str = "resident bytes exceed exact BMF numeric range";

/// Aggregation must never publish a metric without observations.
pub(super) const EMPTY_METRIC: &str = "empty metric sample group";

/// The comparison evidence schema written beside timing output.
pub(super) const BENCHMARK_COMPARISON_V1: &str = "benchmark-comparison/v1";
/// The diagnostics contract.
pub(super) const DIAGNOSTICS: &str = "diagnostics";
/// The complete-report contract.
pub(super) const COMPLETE_REPORT: &str = "complete-report";
/// Both reports agree exactly on a dimension.
pub(super) const EXACT: &str = "exact";
/// The reports differ on a dimension.
pub(super) const DIFFERENT: &str = "different";
/// A dimension cannot be compared.
pub(super) const INCOMPARABLE: &str = "incomparable";

/// The prefix of the benchmark of each rule's verification times.
pub(super) const VERIFY: &str = "verify";

/// The measure of a rule's verification: how long all its cases took, in milliseconds.
pub(super) const TOTAL_MS: &str = "total-ms";

/// The measure of a rule's verification: how long its slowest case took, in milliseconds.
pub(super) const MAX_CASE_MS: &str = "max-case-ms";

/// The measure of a rule's verification: how long its query took to compile, in milliseconds.
pub(super) const COMPILE_MS: &str = "compile-ms";

/// The prefix of the benchmark of each shape's growth, after the language.
pub(super) const GROWTH: &str = "growth";
/// The measure of a shape's growth: the CPU time at the whole size over the floored time at the half size.
pub(super) const DOUBLING_RATIO: &str = "doubling-ratio";
/// The measure of a shape's growth: the CPU time at the whole size, in milliseconds.
pub(super) const LARGER_MS: &str = "larger-ms";
/// The measure of a project's cold run: the candidate's CPU time, in seconds.
pub(super) const CPU_S: &str = "cpu-s";
/// The measure of a project's cold run: the candidate's wall time, in seconds.
pub(super) const WALL_S: &str = "wall-s";
/// The measure of a project's cold run: the candidate's peak resident memory, in bytes.
pub(super) const PEAK_RSS_BYTES: &str = "peak-rss-bytes";
/// The measure of a project's warm run: the candidate's CPU time, in seconds.
pub(super) const WARM_CPU_S: &str = "warm-cpu-s";
/// The measure of a project's warm run: the candidate's peak resident memory, in bytes.
pub(super) const WARM_PEAK_RSS_BYTES: &str = "warm-peak-rss-bytes";
/// The measure of a project's edited run: the candidate's CPU time, in seconds.
pub(super) const EDITED_CPU_S: &str = "edited-cpu-s";
/// The measure of a project's edited run: the candidate's peak resident memory, in bytes.
pub(super) const EDITED_PEAK_RSS_BYTES: &str = "edited-peak-rss-bytes";
/// The measure of a project's store: the bytes of every file the cold run left in the candidate's state.
pub(super) const STORE_BYTES: &str = "store-bytes";
/// The measure of a project's store: how many files the cold run left in the candidate's state.
pub(super) const STORE_FILES: &str = "store-files";
/// The measures the same tree gives again on any machine of the runner's kind: what the cold run leaves in its store,
/// and the peak memory of each run. Every other measure the document writes is in `TIMING`.
pub(super) const DETERMINISTIC: [&str; 5] = [
    STORE_BYTES,
    STORE_FILES,
    PEAK_RSS_BYTES,
    WARM_PEAK_RSS_BYTES,
    EDITED_PEAK_RSS_BYTES,
];
/// The measures of time, which move with the machine and its load.
pub(super) const TIMING: [&str; 9] = [
    CPU_S,
    WALL_S,
    WARM_CPU_S,
    EDITED_CPU_S,
    TOTAL_MS,
    MAX_CASE_MS,
    COMPILE_MS,
    DOUBLING_RATIO,
    LARGER_MS,
];
/// The refusal of a measure neither `DETERMINISTIC` nor `TIMING` names.
pub(super) const UNCLASSED_MEASURE: &str = "a measure that is neither deterministic nor timing";
/// The refusal of a selection that keeps no measure.
pub(super) const NOTHING_SELECTED: &str = "the document holds no measure of the class selected";
/// The field of a measure that holds its value.
pub(super) const VALUE: &str = "value";
/// The refusal of a benchmark added twice to one document.
pub(super) const DUPLICATE_BENCHMARK: &str = "a benchmark is added to the document twice";
/// The refusal of a verification times file without its rule's times.
pub(super) const MISSING_VERIFICATION: &str = "no verification times";
/// The refusal of a second document from one run.
pub(super) const FINISHED: &str = "the document is already written";
