//! The file names of one verification's native receipt directory.
/// The native case receipt rows.
pub const CASES_JSONL: &str = "cases.jsonl";
/// The exact standard output of a captured process.
pub const STDOUT_BIN: &str = "stdout.bin";
/// The exact standard error of a captured process.
pub const STDERR_BIN: &str = "stderr.bin";
/// The raw invocation record of a captured process.
pub const INVOCATION_JSON: &str = "invocation.json";
/// The explicit receipt destination every registered verifier accepts, followed by its path.
pub const CASE_RECEIPTS_ARGUMENT: &str = "--case-receipts=";
/// The row field naming the sealed identity of a case's sources.
pub const SOURCE_SHA256: &str = "source_sha256";
/// The row field saying whether an observation completed.
pub const COMPLETE: &str = "complete";
/// The row field holding what a native side reported.
pub const DIAGNOSTICS: &str = "diagnostics";
/// The row value naming the tool under measurement.
pub const CANDIDATE: &str = "candidate";
/// The command that verifies registered cases.
pub const VERIFY_CASES: &str = "verify-cases";
/// The schema label of a public inventory summary.
pub const BENCHMARK_INVENTORY_V1: &str = "benchmark-inventory/v1";
/// The state of a summary whose rules are still being verified.
pub const RUNNING: &str = "running";
/// The state of a run refused before every rule had a row.
pub const FAILED: &str = "failed";
/// The status of a rule that verified.
pub const PASS: &str = "PASS";
/// The status of a rule that failed, was missing or was not run.
pub const FAIL: &str = "FAIL";
