//! Artifact filenames, executable protocol and refusal vocabulary.
/// Shared filename for tool profiles and language workload manifests.
pub(super) const MANIFEST: &str = "benchmark.toml";
/// Directory containing authored rule definitions.
pub(super) const RULES: &str = "rules";
/// Executable RyoQL source filename within one rule directory.
pub(super) const QUERY: &str = "query.ryoql";
/// Independent fixture filename within one rule directory.
pub(super) const CASES: &str = "cases.toml";
/// Reference tool argument used to verify its reported version.
pub(super) const VERSION_ARGUMENT: &str = "--version";
/// Audit refusal when a rule directory name is not UTF-8.
pub(super) const INVALID_RULE_ID: &str = "non-UTF8 rule id";
/// Fingerprint refusal for a non-UTF-8 artifact name.
pub(super) const INVALID_EVIDENCE_PATH: &str = "non-UTF8 evidence path";
/// Fingerprint refusal for non-regular artifact entries.
pub(super) const INVALID_EVIDENCE_KIND: &str =
    "evidence paths must be regular files or directories";
/// Executable digest refusal for a non-regular file.
pub(super) const INVALID_DIGEST_KIND: &str = "executable digest requires a regular file";
/// Refusal when executable bytes differ from the pinned digest.
pub(super) const TOOL_HASH_MISMATCH: &str = "tool executable hash differs from pin";
/// Refusal when the executable reports a different version.
pub(super) const TOOL_VERSION_MISMATCH: &str = "tool version differs from pin";
/// Refusal when a canonical patch path leaves the shared workload root.
pub(super) const PATCH_ESCAPE: &str = "patch escapes the shared suite directory";
/// Refusal when a case source path has no materializable parent.
pub(super) const MISSING_CASE_PARENT: &str = "missing case parent";
/// Text of the module source in this part.
pub(super) const MODULE_SOURCE: &str = "module source";
/// Text of the profile must be a table in this part.
pub(super) const PROFILE_MUST_BE_A_TABLE: &str = "profile must be a table";
/// Text of the defaults in this part.
pub(super) const DEFAULTS: &str = "defaults";
/// Text of the supporting rules in this part.
pub(super) const SUPPORTING_RULES: &str = "supporting-rules";
/// Text of the pending in this part.
pub(super) const PENDING: &str = "pending";
/// Text of the deferred in this part.
pub(super) const DEFERRED: &str = "deferred";
/// Text of the rule observations in this part.
pub(super) const RULE_OBSERVATIONS: &str = "rule-observations";
/// Text of the rule applicability in this part.
pub(super) const RULE_APPLICABILITY: &str = "rule-applicability";
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
pub(super) const STDOUT_BIN: &str = "stdout.bin";
/// Text of the stderr in this part.
pub(super) const STDERR: &str = "stderr";
/// Text of the stderr bin in this part.
pub(super) const STDERR_BIN: &str = "stderr.bin";
/// Text of the invocation in this part.
pub(super) const INVOCATION: &str = "invocation";
/// Text of the invocation json in this part.
pub(super) const INVOCATION_JSON: &str = "invocation.json";
/// Text of the raw artifact path is not utf8 in this part.
pub(super) const RAW_ARTIFACT_PATH_IS_NOT_UTF8: &str = "raw artifact path is not UTF8";
/// Text of the slash in this part.
pub(super) const SLASH: &str = "/";
/// Text of the path in this part.
pub(super) const PATH: &str = "path";
/// Text of the sha256 in this part.
pub(super) const SHA256: &str = "sha256";
/// Text of the process observation in this part.
pub(super) const PROCESS_OBSERVATION: &str = "process_observation";
/// Text of the artifacts in this part.
pub(super) const ARTIFACTS: &str = "artifacts";
/// Text of the rule in this part.
pub(super) const RULE: &str = "rule";
/// Text of the check001 in this part.
pub(super) const CHECK001: &str = "CHECK001";
/// Text of the name in this part.
pub(super) const NAME: &str = "name";
/// Text of the one in this part.
pub(super) const ONE: &str = "one";
/// Text of the side in this part.
pub(super) const SIDE: &str = "side";
/// Text of the reference in this part.
pub(super) const REFERENCE: &str = "reference";
/// Text of the kind in this part.
pub(super) const KIND: &str = "kind";
/// Text of the process failure in this part.
pub(super) const PROCESS_FAILURE: &str = "process-failure";
/// Text of the a in this part.
pub(super) const A: &str = "a";
/// Text of the exit in this part.
pub(super) const EXIT: &str = "exit";
/// Text of the error in this part.
pub(super) const ERROR: &[u8] = b"error\n";
/// Text of the n 001 cases jsonl in this part.
pub(super) const N_001_CASES_JSONL: &str = "001.cases.jsonl";
/// Text of the n 001 cases jsonl process orphan in this part.
pub(super) const N_001_CASES_JSONL_PROCESS_ORPHAN: &str = "001.cases.jsonl.process-orphan";
/// Text of the second in this part.
pub(super) const SECOND: &[u8] = b"second";
/// Text of the first in this part.
pub(super) const FIRST: &[u8] = b"first";
/// Text of the n 002 cases jsonl in this part.
pub(super) const N_002_CASES_JSONL: &str = "002.cases.jsonl";
/// Text of the cases jsonl in this part.
pub(super) const CASES_JSONL: &str = "cases.jsonl";
/// Text of the changed in this part.
pub(super) const CHANGED: &[u8] = b"changed";
/// Text of the unexpected bin in this part.
pub(super) const UNEXPECTED_BIN: &str = "unexpected.bin";
/// Text of the extra in this part.
pub(super) const EXTRA: &[u8] = b"extra";
/// Text of the bytes in this part.
pub(super) const BYTES: &str = "bytes";
/// Text of the non utf8 artifact path in this part.
pub(super) const NON_UTF8_ARTIFACT_PATH: &str = "non-UTF8 artifact path";
/// Text of the unsafe artifact relative path in this part.
pub(super) const UNSAFE_ARTIFACT_RELATIVE_PATH: &str = "unsafe artifact-relative path";
/// Text of the non utf8 rule directory in this part.
pub(super) const NON_UTF8_RULE_DIRECTORY: &str = "non-UTF8 rule directory";
/// Text of the stale reference discrepancy source identity in this part.
pub(super) const STALE_REFERENCE_DISCREPANCY_SOURCE_IDENTITY: &str = "stale reference discrepancy source identity";
/// Text of the undeclared reference discrepancy observation in this part.
pub(super) const UNDECLARED_REFERENCE_DISCREPANCY_OBSERVATION: &str = "undeclared reference discrepancy observation scope";
/// Text of the stale reference discrepancy file metadata id in this part.
pub(super) const STALE_REFERENCE_DISCREPANCY_FILE_METADATA_ID: &str = "stale reference discrepancy file metadata identity";
/// Text of the stale or external discrepancy evidence in this part.
pub(super) const STALE_OR_EXTERNAL_DISCREPANCY_EVIDENCE: &str = "stale or external discrepancy evidence";
/// Text of the stale discrepancy evidence bytes in this part.
pub(super) const STALE_DISCREPANCY_EVIDENCE_BYTES: &str = "stale discrepancy evidence bytes";
/// Text of the tool contract evidence does not establish in this part.
pub(super) const TOOL_CONTRACT_EVIDENCE_DOES_NOT_ESTABLISH: &str = "tool-contract evidence does not establish its declared authority";
/// Text of the schema in this part.
pub(super) const SCHEMA: &str = "schema";
/// Text of the public tool contract v1 in this part.
pub(super) const PUBLIC_TOOL_CONTRACT_V1: &str = "public-tool-contract/v1";
/// Text of the source revision in this part.
pub(super) const SOURCE_REVISION: &str = "source_revision";
/// Text of the source path in this part.
pub(super) const SOURCE_PATH: &str = "source_path";
/// Text of the source text in this part.
pub(super) const SOURCE_TEXT: &str = "source_text";
/// Text of the contract assertion in this part.
pub(super) const CONTRACT_ASSERTION: &str = "contract_assertion";
/// Text of the source sha256 in this part.
pub(super) const SOURCE_SHA256: &str = "source_sha256";
/// Text of the prior worker control v1 in this part.
pub(super) const PRIOR_WORKER_CONTROL_V1: &str = "prior-worker-control/v1";
/// Text of the worker sha256 in this part.
pub(super) const WORKER_SHA256: &str = "worker_sha256";
/// Text of the control source sha256 in this part.
pub(super) const CONTROL_SOURCE_SHA256: &str = "control_source_sha256";
/// Text of the control query sha256 in this part.
pub(super) const CONTROL_QUERY_SHA256: &str = "control_query_sha256";
/// Text of the control profile sha256 in this part.
pub(super) const CONTROL_PROFILE_SHA256: &str = "control_profile_sha256";
/// Text of the witness case in this part.
pub(super) const WITNESS_CASE: &str = "witness_case";
/// Text of the control query in this part.
pub(super) const CONTROL_QUERY: &str = "control_query";
/// Text of the control profile in this part.
pub(super) const CONTROL_PROFILE: &str = "control_profile";
/// Text of the control sources in this part.
pub(super) const CONTROL_SOURCES: &str = "control_sources";
/// Text of the raw receipt in this part.
pub(super) const RAW_RECEIPT: &str = "raw_receipt";
/// Text of the raw receipt sha256 in this part.
pub(super) const RAW_RECEIPT_SHA256: &str = "raw_receipt_sha256";
/// Text of the execution manifest in this part.
pub(super) const EXECUTION_MANIFEST: &str = "execution_manifest";
/// Text of the execution manifest sha256 in this part.
pub(super) const EXECUTION_MANIFEST_SHA256: &str = "execution_manifest_sha256";
/// Text of the prior worker execution v1 in this part.
pub(super) const PRIOR_WORKER_EXECUTION_V1: &str = "prior-worker-execution/v1";
/// Text of the status in this part.
pub(super) const STATUS: &str = "status";
/// Text of the command in this part.
pub(super) const COMMAND: &str = "command";
/// Text of the verify cases in this part.
pub(super) const VERIFY_CASES: &str = "verify-cases";
/// Text of the stage root in this part.
pub(super) const STAGE_ROOT: &str = "stage_root";
/// Text of the candidate in this part.
pub(super) const CANDIDATE: &str = "candidate";
/// Text of the complete in this part.
pub(super) const COMPLETE: &str = "complete";
/// Text of the error 2 in this part.
pub(super) const ERROR_2: &str = "error";
/// Text of the case passed in this part.
pub(super) const CASE_PASSED: &str = "case_passed";
/// Text of the skipped in this part.
pub(super) const SKIPPED: &str = "skipped";
/// Text of the diagnostics in this part.
pub(super) const DIAGNOSTICS: &str = "diagnostics";
/// Text of the expected in this part.
pub(super) const EXPECTED: &str = "expected";
/// Text of the coverage in this part.
pub(super) const COVERAGE: &str = "coverage";
/// Text of the selected rule file pairs in this part.
pub(super) const SELECTED_RULE_FILE_PAIRS: &str = "selected_rule_file_pairs";
/// Text of the evaluated rule file pairs in this part.
pub(super) const EVALUATED_RULE_FILE_PAIRS: &str = "evaluated_rule_file_pairs";
/// Text of the unknown or unread rule file pairs in this part.
pub(super) const UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: &str = "unknown_or_unread_rule_file_pairs";
/// Text of the skipped rule file pairs in this part.
pub(super) const SKIPPED_RULE_FILE_PAIRS: &str = "skipped_rule_file_pairs";
/// Text of the raw artifact escapes its profile in this part.
pub(super) const RAW_ARTIFACT_ESCAPES_ITS_PROFILE: &str = "raw artifact escapes its profile";
/// Text of the raw artifact byte identity differs in this part.
pub(super) const RAW_ARTIFACT_BYTE_IDENTITY_DIFFERS: &str = "raw artifact byte identity differs";
/// Text of the duplicate raw artifact path in this part.
pub(super) const DUPLICATE_RAW_ARTIFACT_PATH: &str = "duplicate raw artifact path";
/// Text of the source artifact escapes its boundary in this part.
pub(super) const SOURCE_ARTIFACT_ESCAPES_ITS_BOUNDARY: &str = "source artifact escapes its boundary";
/// Text of the nonutf8 source path in this part.
pub(super) const NONUTF8_SOURCE_PATH: &str = "nonUTF8 source path";
/// Text of the metadata source is not a regular in this part.
pub(super) const METADATA_SOURCE_IS_NOT_A_REGULAR: &str = "metadata source is not a regular file";
/// Text of the posix executable observations are unsupporte in this part.
pub(super) const POSIX_EXECUTABLE_OBSERVATIONS_ARE_UNSUPPORTE: &str = "POSIX executable observations are unsupported on this host";
/// Text of the staged executable metadata differs in this part.
pub(super) const STAGED_EXECUTABLE_METADATA_DIFFERS: &str = "staged executable metadata differs";
/// Text of the metadata names an undeclared source in this part.
pub(super) const METADATA_NAMES_AN_UNDECLARED_SOURCE: &str = "metadata names an undeclared source";
/// Text of the metadata source escapes input root in this part.
pub(super) const METADATA_SOURCE_ESCAPES_INPUT_ROOT: &str = "metadata source escapes input root";
/// Text of the declared executable metadata drifted before in this part.
pub(super) const DECLARED_EXECUTABLE_METADATA_DRIFTED_BEFORE: &str = "declared executable metadata drifted before prepare";
/// Text of the metadata source name is not utf8 in this part.
pub(super) const METADATA_SOURCE_NAME_IS_NOT_UTF8: &str = "metadata source name is not UTF8";
/// Text of the duplicate physical metadata source in this part.
pub(super) const DUPLICATE_PHYSICAL_METADATA_SOURCE: &str = "duplicate physical metadata source";
/// Text of the missing physical metadata digest in this part.
pub(super) const MISSING_PHYSICAL_METADATA_DIGEST: &str = "missing physical metadata digest";
/// Text of the given project file entry lacks staged in this part.
pub(super) const GIVEN_PROJECT_FILE_ENTRY_LACKS_STAGED: &str = "Given project file entry lacks staged source bytes";
/// Text of the given project directory collides with a in this part.
pub(super) const GIVEN_PROJECT_DIRECTORY_COLLIDES_WITH_A: &str = "Given project directory collides with a source file";
/// Text of the given project missing entry collides with in this part.
pub(super) const GIVEN_PROJECT_MISSING_ENTRY_COLLIDES_WITH: &str = "Given project missing entry collides with staged context";
/// Text of the given project non file entry requires in this part.
pub(super) const GIVEN_PROJECT_NON_FILE_ENTRY_REQUIRES: &str = "Given project non-file entry requires a materialization contract";
