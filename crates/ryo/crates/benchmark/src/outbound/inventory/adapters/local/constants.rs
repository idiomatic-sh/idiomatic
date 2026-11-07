//! The text this part uses.
/// Text of the summary json in this part.
pub(super) const SUMMARY_JSON: &str = "summary.json";
/// Text of the result json in this part.
pub(super) const RESULT_JSON: &str = "result.json";
/// Text of the inventory output already contains a summary in this part.
pub(super) const INVENTORY_OUTPUT_ALREADY_CONTAINS_A_SUMMARY: &str =
    "inventory output already contains a summary";
/// Text of the inventory rule output already contains recei in this part.
pub(super) const INVENTORY_RULE_OUTPUT_ALREADY_CONTAINS_RECEI: &str =
    "inventory rule output already contains receipts";
/// Text of the registered context must be canonical and in this part.
pub(super) const REGISTERED_CONTEXT_MUST_BE_CANONICAL_AND: &str =
    "registered context must be canonical and absolute";
/// Text of the registered context contains non regular entr in this part.
pub(super) const REGISTERED_CONTEXT_CONTAINS_NON_REGULAR_ENTR: &str =
    "registered context contains non-regular entry";
/// Text of the receipt filename is not utf8 in this part.
pub(super) const RECEIPT_FILENAME_IS_NOT_UTF8: &str = "receipt filename is not UTF8";
/// Text of the unsafe raw process artifact path in this part.
pub(super) const UNSAFE_RAW_PROCESS_ARTIFACT_PATH: &str = "unsafe raw process artifact path";
/// Text of the nonutf8 attempt in this part.
pub(super) const NONUTF8_ATTEMPT: &str = "nonUTF8 attempt";
/// Text of the nonutf8 case key in this part.
pub(super) const NONUTF8_CASE_KEY: &str = "nonUTF8 case key";
/// The length of a lowercase hexadecimal SHA-256 digest.
pub(super) const DIGEST_HEX_LENGTH: usize = 64;
/// Text of the raw artifact is outside its receipt in this part.
pub(super) const RAW_ARTIFACT_IS_OUTSIDE_ITS_RECEIPT: &str =
    "raw artifact is outside its receipt-file namespace";
/// Text of the raw artifact escapes its receipt boundary in this part.
pub(super) const RAW_ARTIFACT_ESCAPES_ITS_RECEIPT_BOUNDARY: &str =
    "raw artifact escapes its receipt boundary";
/// Text of the raw process artifact is stale or in this part.
pub(super) const RAW_PROCESS_ARTIFACT_IS_STALE_OR: &str =
    "raw process artifact is stale or reused by another observation";
/// Text of the raw artifact has no case directory in this part.
pub(super) const RAW_ARTIFACT_HAS_NO_CASE_DIRECTORY: &str = "raw artifact has no case directory";
/// Text of the extra raw artifacts in referenced case in this part.
pub(super) const EXTRA_RAW_ARTIFACTS_IN_REFERENCED_CASE: &str =
    "extra raw artifacts in referenced case directory";
/// Text of an invocation record that names no process identity.
pub(super) const INVOCATION_LACKS_PROCESS_IDENTITY: &str =
    "invocation record lacks its complete process identity";
/// The schema label of a public inventory summary.
pub(super) const BENCHMARK_INVENTORY_V1: &str = "benchmark-inventory/v1";
/// The state of a summary whose rules are still being verified.
pub(super) const RUNNING: &str = "running";
/// The state of a summary that has a row for every rule.
pub(super) const COMPLETE: &str = "complete";
/// The state of a run refused before every rule had a row.
pub(super) const FAILED: &str = "failed";
/// The status of a rule that verified.
pub(super) const PASS: &str = "PASS";
/// The status of a rule that failed, was missing or was not run.
pub(super) const FAIL: &str = "FAIL";
/// The newline that ends a summary file.
pub(super) const NEWLINE: &[u8] = b"\n";
