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
