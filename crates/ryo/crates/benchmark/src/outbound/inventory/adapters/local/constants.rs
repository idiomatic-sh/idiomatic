//! The text this part uses.
/// Text of the newline in this part.
pub(super) const NEWLINE: &[u8] = b"\n";
/// Text of the summary json in this part.
pub(super) const SUMMARY_JSON: &str = "summary.json";
/// Text of the stdout bin in this part.
pub(super) const STDOUT_BIN: &str = "stdout.bin";
/// Text of the stderr bin in this part.
pub(super) const STDERR_BIN: &str = "stderr.bin";
/// Text of the result json in this part.
pub(super) const RESULT_JSON: &str = "result.json";
/// Text of the inventory output already contains a summary in this part.
pub(super) const INVENTORY_OUTPUT_ALREADY_CONTAINS_A_SUMMARY: &str =
    "inventory output already contains a summary";
/// Text of the cases jsonl in this part.
pub(super) const CASES_JSONL: &str = "cases.jsonl";
/// Text of the inventory rule output already contains recei in this part.
pub(super) const INVENTORY_RULE_OUTPUT_ALREADY_CONTAINS_RECEI: &str =
    "inventory rule output already contains receipts";
/// Text of the receipt only registration cannot execute in this part.
pub(super) const RECEIPT_ONLY_REGISTRATION_CANNOT_EXECUTE: &str =
    "receipt-only registration cannot execute";
/// Text of the benchmark toml in this part.
pub(super) const BENCHMARK_TOML: &str = "benchmark.toml";
/// Text of the registered case context is empty in this part.
pub(super) const REGISTERED_CASE_CONTEXT_IS_EMPTY: &str = "registered case context is empty";
/// Text of the registered context must be canonical and in this part.
pub(super) const REGISTERED_CONTEXT_MUST_BE_CANONICAL_AND: &str =
    "registered context must be canonical and absolute";
/// Text of the registered context contains non regular entr in this part.
pub(super) const REGISTERED_CONTEXT_CONTAINS_NON_REGULAR_ENTR: &str =
    "registered context contains non-regular entry";
/// Text of the registered context namespace differs from pi in this part.
pub(super) const REGISTERED_CONTEXT_NAMESPACE_DIFFERS_FROM_PI: &str =
    "registered context namespace differs from pinned artifacts";
/// Text of the inventorylocaladapter in this part.
pub(super) const INVENTORYLOCALADAPTER: &str = "InventoryLocalAdapter";
/// Text of the receipt filename is not utf8 in this part.
pub(super) const RECEIPT_FILENAME_IS_NOT_UTF8: &str = "receipt filename is not UTF8";
/// Text of the dot in this part.
pub(super) const DOT: &str = ".";
/// Text of the invocation json in this part.
pub(super) const INVOCATION_JSON: &str = "invocation.json";
/// Text of the unsafe raw process artifact path in this part.
pub(super) const UNSAFE_RAW_PROCESS_ARTIFACT_PATH: &str = "unsafe raw process artifact path";
/// Text of the nonutf8 attempt in this part.
pub(super) const NONUTF8_ATTEMPT: &str = "nonUTF8 attempt";
/// Text of the nonutf8 case key in this part.
pub(super) const NONUTF8_CASE_KEY: &str = "nonUTF8 case key";
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
