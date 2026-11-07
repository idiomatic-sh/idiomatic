//! The file names and refusal text of the receipt writer.
/// Text of the receipt filename must be utf8 in this part.
pub(super) const RECEIPT_FILENAME_MUST_BE_UTF8: &str = "receipt filename must be UTF8";
/// Text of the dot in this part.
pub(super) const DOT: &str = ".";
/// Text of the unsafe receipt file identity in this part.
pub(super) const UNSAFE_RECEIPT_FILE_IDENTITY: &str = "unsafe receipt file identity";
/// Text of the invalid framed process case key in this part.
pub(super) const INVALID_FRAMED_PROCESS_CASE_KEY: &str = "invalid framed process case key";
/// Text of the stdout in this part.
pub(super) const STDOUT: &str = "stdout";
/// Text of the stdout bin in this part.
pub(super) const STDOUT_BIN: &str =
    ryo_benchmark_contracts::domain::receipts::constants::STDOUT_BIN;
/// Text of the stderr in this part.
pub(super) const STDERR: &str = "stderr";
/// Text of the stderr bin in this part.
pub(super) const STDERR_BIN: &str =
    ryo_benchmark_contracts::domain::receipts::constants::STDERR_BIN;
/// Text of the invocation in this part.
pub(super) const INVOCATION: &str = "invocation";
/// Text of the invocation json in this part.
pub(super) const INVOCATION_JSON: &str =
    ryo_benchmark_contracts::domain::receipts::constants::INVOCATION_JSON;
/// Text of the raw artifact path is not utf8 in this part.
pub(super) const RAW_ARTIFACT_PATH_IS_NOT_UTF8: &str = "raw artifact path is not UTF8";
/// Text of the slash in this part.
pub(super) const SLASH: &str = "/";
/// Refuse a process observation that has no raw artifacts to point at.
pub(super) const PROCESS_OBSERVATION_NEEDS_RAW_ARTIFACTS: &str =
    "a process observation needs its three raw artifacts";
