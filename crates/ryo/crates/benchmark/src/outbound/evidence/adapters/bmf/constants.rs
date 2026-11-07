//! Evidence stream protocol.
/// Refusal when the injected evidence writer lock cannot be recovered.
pub(super) const POISONED: &str = "evidence output lock poisoned";
/// One complete BMF document is emitted per output line.
pub(super) const NEWLINE: &[u8] = b"\n";

/// Largest integer byte count that the BMF floating metric can represent exactly.
pub(super) const MAX_EXACT_RESIDENT: u64 = 1_u64 << f64::MANTISSA_DIGITS;
/// Refuse evidence that would silently round a resident-byte observation.
pub(super) const INEXACT_RESIDENT: &str = "resident bytes exceed exact BMF numeric range";

/// Full performance evidence requires both child CPU and resident-memory observations.
pub(super) const MISSING_RESOURCES: &str = "resource measurements absent";

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
