//! Stable operation vocabulary and artifact identities.
/// Refuse a contestant report that does not cover exactly the requested files.
pub(super) const INPUT_COVERAGE: &str = "tool input coverage differs from requested corpus";
/// Reference contestant label in differential failures.
pub(super) const REFERENCE: &str = "reference";
/// Candidate contestant label in differential failures.
pub(super) const CANDIDATE: &str = "candidate";
/// Separate complete fixture failure records.
pub(super) const LINE_SEPARATOR: &str = "\n";

/// Authored rule directory within the frozen profile.

/// Refuse certification without executable cases.
pub(super) const NO_CASES: &str = "no executable rule cases";
/// Text of the process root is not utf8 in this part.
pub(super) const PROCESS_ROOT_IS_NOT_UTF8: &str = "process root is not UTF8";
/// Text of the process state is not utf8 in this part.
pub(super) const PROCESS_STATE_IS_NOT_UTF8: &str = "process state is not UTF8";
/// Text of the missing profile boundary in this part.
pub(super) const MISSING_PROFILE_BOUNDARY: &str = "missing profile boundary";
/// Text of the actual request root in this part.
pub(super) const ACTUAL_REQUEST_ROOT: &str = "actual_request_root";
/// Text of the files in this part.
pub(super) const FILES: &str = "files";
/// Text of the executable sha256 in this part.
pub(super) const EXECUTABLE_SHA256: &str = "executable_sha256";
/// Text of the settings sha256 in this part.
pub(super) const SETTINGS_SHA256: &str = "settings_sha256";
/// Text of the normalized settings in this part.
pub(super) const NORMALIZED_SETTINGS: &str = "normalized_settings";
/// Text of the profile sha256 in this part.
pub(super) const PROFILE_SHA256: &str = "profile_sha256";
/// Text of the benchmark toml in this part.
pub(super) const BENCHMARK_TOML: &str = "benchmark.toml";
/// Text of the tool in this part.
pub(super) const TOOL: &str = "tool";
/// Text of the version in this part.
pub(super) const VERSION: &str = "version";
/// Text of the state in this part.
pub(super) const STATE: &str = "state";
/// Text of the arguments in this part.
pub(super) const ARGUMENTS: &str = "arguments";
/// Text of the environment in this part.
pub(super) const ENVIRONMENT: &str = "environment";
/// Text of the directory in this part.
pub(super) const DIRECTORY: &str = "directory";
/// Text of the exit in this part.
pub(super) const EXIT: &str = "exit";
/// Text of the signal in this part.
pub(super) const SIGNAL: &str = "signal";
/// Text of the observer no deadline in this part.
pub(super) const OBSERVER_NO_DEADLINE: &str = "observer_no_deadline";
/// Text of the file metadata sha256 in this part.
pub(super) const FILE_METADATA_SHA256: &str = "file_metadata_sha256";
/// Text of the metadata error in this part.
pub(super) const METADATA_ERROR: &str = "metadata_error";
/// Text of the contestant does not implement the declared in this part.
pub(super) const CONTESTANT_DOES_NOT_IMPLEMENT_THE_DECLARED: &str =
    "contestant does not implement the declared rule configuration contract";
/// Text of the rule in this part.
pub(super) const RULE: &str = "rule";
/// Text of the path in this part.
pub(super) const PATH: &str = "path";
/// Text of the reason in this part.
pub(super) const REASON: &str = "reason";
/// Text of the syntax rejected in this part.
pub(super) const SYNTAX_REJECTED: &str = "syntax-rejected";
/// Text of the mode in this part.
pub(super) const MODE: &str = "mode";
/// Text of the listed disk in this part.
pub(super) const LISTED_DISK: &str = "listed-disk";
/// Text of the logical root in this part.
pub(super) const LOGICAL_ROOT: &str = "logical_root";
/// Text of the actual root in this part.
pub(super) const ACTUAL_ROOT: &str = "actual_root";
/// Text of the manifest hash in this part.
pub(super) const MANIFEST_HASH: &str = "manifest_hash";
/// Text of the observation schema in this part.
pub(super) const OBSERVATION_SCHEMA: &str = "observation_schema";
/// Text of the benchmark case v2 in this part.
pub(super) const BENCHMARK_CASE_V2: &str = "benchmark-case/v2";
/// Text of the expectation contract in this part.
pub(super) const EXPECTATION_CONTRACT: &str = "expectation_contract";
/// Text of the diagnostics in this part.
pub(super) const DIAGNOSTICS: &str = "diagnostics";
/// Text of the execution metadata in this part.
pub(super) const EXECUTION_METADATA: &str = "execution_metadata";
/// Text of the name in this part.
pub(super) const NAME: &str = "name";
/// Text of the side in this part.
pub(super) const SIDE: &str = "side";
/// Text of the skipped in this part.
pub(super) const SKIPPED: &str = "skipped";
/// Text of the coverage in this part.
pub(super) const COVERAGE: &str = "coverage";
/// Text of the selected rule file pairs in this part.
pub(super) const SELECTED_RULE_FILE_PAIRS: &str = "selected_rule_file_pairs";
/// Text of the evaluated rule file pairs in this part.
pub(super) const EVALUATED_RULE_FILE_PAIRS: &str = "evaluated_rule_file_pairs";
/// Text of the skipped rule file pairs in this part.
pub(super) const SKIPPED_RULE_FILE_PAIRS: &str = "skipped_rule_file_pairs";
/// Text of the unknown or unread rule file pairs in this part.
pub(super) const UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: &str = "unknown_or_unread_rule_file_pairs";
/// Text of the context manifest in this part.
pub(super) const CONTEXT_MANIFEST: &str = "context_manifest";
/// Text of the project context in this part.
pub(super) const PROJECT_CONTEXT: &str = "project_context";
/// Text of the expected in this part.
pub(super) const EXPECTED: &str = "expected";
/// Text of the reference expected in this part.
pub(super) const REFERENCE_EXPECTED: &str = "reference_expected";
/// Text of the documented reference discrepancy in this part.
pub(super) const DOCUMENTED_REFERENCE_DISCREPANCY: &str = "documented_reference_discrepancy";
/// Text of the documented tool contract difference in this part.
pub(super) const DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: &str = "documented_tool_contract_difference";
/// Text of the discrepancy validation in this part.
pub(super) const DISCREPANCY_VALIDATION: &str = "discrepancy_validation";
/// Text of the raw parity matches intended in this part.
pub(super) const RAW_PARITY_MATCHES_INTENDED: &str = "raw_parity_matches_intended";
/// Text of the diagnostics match intended in this part.
pub(super) const DIAGNOSTICS_MATCH_INTENDED: &str = "diagnostics_match_intended";
/// Text of the source sha256 in this part.
pub(super) const SOURCE_SHA256: &str = "source_sha256";
/// Text of the project inputs sha256 in this part.
pub(super) const PROJECT_INPUTS_SHA256: &str = "project_inputs_sha256";
/// Text of the error in this part.
pub(super) const ERROR: &str = "error";
/// Text of the case passed in this part.
pub(super) const CASE_PASSED: &str = "case_passed";
/// Text of the complete in this part.
pub(super) const COMPLETE: &str = "complete";
/// Text of the rule context in this part.
pub(super) const RULE_CONTEXT: &str = "rule_context";
/// Text of the rule observations in this part.
pub(super) const RULE_OBSERVATIONS: &str = "rule_observations";
/// Text of the selected rules in this part.
pub(super) const SELECTED_RULES: &str = "selected_rules";
/// Text of the process observation in this part.
pub(super) const PROCESS_OBSERVATION: &str = "process_observation";
/// Text of the kind in this part.
pub(super) const KIND: &str = "kind";
/// Text of the process failure in this part.
pub(super) const PROCESS_FAILURE: &str = "process-failure";
/// Text of the template sha256 in this part.
pub(super) const TEMPLATE_SHA256: &str = "template_sha256";
/// Text of the missing case profile boundary in this part.
pub(super) const MISSING_CASE_PROFILE_BOUNDARY: &str = "missing case profile boundary";
/// Text of the file in this part.
pub(super) const FILE: &str = "file";
/// Text of the start in this part.
pub(super) const START: &str = "start";
/// Text of the end in this part.
pub(super) const END: &str = "end";
/// Text of the discrepancy basis in this part.
pub(super) const DISCREPANCY_BASIS: &str = "discrepancy_basis";
/// Text of the language defect in this part.
pub(super) const LANGUAGE_DEFECT: &str = "language-defect";
/// Text of the tool contract difference in this part.
pub(super) const TOOL_CONTRACT_DIFFERENCE: &str = "tool-contract-difference";
/// Text of the process observation group sha256 in this part.
pub(super) const PROCESS_OBSERVATION_GROUP_SHA256: &str = "process_observation_group_sha256";
/// Text of the reference expectation kind in this part.
pub(super) const REFERENCE_EXPECTATION_KIND: &str = "reference_expectation_kind";
/// Text of the actual executable sha256 in this part.
pub(super) const ACTUAL_EXECUTABLE_SHA256: &str = "actual_executable_sha256";
/// Text of the reference output sha256 in this part.
pub(super) const REFERENCE_OUTPUT_SHA256: &str = "reference_output_sha256";
/// Text of the intended output sha256 in this part.
pub(super) const INTENDED_OUTPUT_SHA256: &str = "intended_output_sha256";
/// Text of the pass in this part.
pub(super) const PASS: &str = "PASS";
/// Text of the fail in this part.
pub(super) const FAIL: &str = "FAIL";
/// Text of the benchmark inventory v1 in this part.
pub(super) const BENCHMARK_INVENTORY_V1: &str = "benchmark-inventory/v1";
/// Text of the running in this part.
pub(super) const RUNNING: &str = "running";
/// Text of the unknown or duplicate registered rule in this part.
pub(super) const UNKNOWN_OR_DUPLICATE_REGISTERED_RULE: &str =
    "unknown or duplicate registered rule";
/// Text of the failed in this part.
pub(super) const FAILED: &str = "failed";
/// Text of the profiles in this part.
pub(super) const PROFILES: &str = "profiles";
/// Text of the missing implementation registration in this part.
pub(super) const MISSING_IMPLEMENTATION_REGISTRATION: &str = "missing implementation registration";
/// Text of the missing implementation invocation in this part.
pub(super) const MISSING_IMPLEMENTATION_INVOCATION: &str = "missing implementation invocation";
/// Text of the executable rule has no tested definition in this part.
pub(super) const EXECUTABLE_RULE_HAS_NO_TESTED_DEFINITION: &str =
    "executable rule has no tested definition pin";
/// Text of the query module pins are outside the in this part.
pub(super) const QUERY_MODULE_PINS_ARE_OUTSIDE_THE: &str =
    "query/module pins are outside the registered case context";
/// Text of the missing independent cases in this part.
pub(super) const MISSING_INDEPENDENT_CASES: &str = "missing independent cases";
/// Text of the case identity belongs to another registered in this part.
pub(super) const CASE_IDENTITY_BELONGS_TO_ANOTHER_REGISTERED: &str =
    "case identity belongs to another registered rule";
/// Text of the case verification requires execution authori in this part.
pub(super) const CASE_VERIFICATION_REQUIRES_EXECUTION_AUTHORI: &str =
    "case verification requires execution authority";
/// Text of the cases jsonl in this part.
pub(super) const CASES_JSONL: &str = "cases.jsonl";
/// Text of the verify cases in this part.
pub(super) const VERIFY_CASES: &str = "verify-cases";
/// Text of the native receipt digest is missing in this part.
pub(super) const NATIVE_RECEIPT_DIGEST_IS_MISSING: &str = "native receipt digest is missing";
/// Text of the receipt only inventory cannot authorize exec in this part.
pub(super) const RECEIPT_ONLY_INVENTORY_CANNOT_AUTHORIZE_EXEC: &str =
    "receipt-only inventory cannot authorize execution";
/// Text of the canonical inventory profile executable ident in this part.
pub(super) const CANONICAL_INVENTORY_PROFILE_EXECUTABLE_IDENT: &str =
    "canonical inventory/profile/executable identity differs";
/// Text of the missing or malformed execution capability in this part.
pub(super) const MISSING_OR_MALFORMED_EXECUTION_CAPABILITY: &str =
    "missing or malformed execution capability";
/// Text of the execution capability is not explicitly decla in this part.
pub(super) const EXECUTION_CAPABILITY_IS_NOT_EXPLICITLY_DECLA: &str =
    "execution capability is not explicitly declared";
/// Text of the missing execution files in this part.
pub(super) const MISSING_EXECUTION_FILES: &str = "missing execution files";
/// Text of the invalid execution file in this part.
pub(super) const INVALID_EXECUTION_FILE: &str = "invalid execution file";
/// Text of the missing execution skips in this part.
pub(super) const MISSING_EXECUTION_SKIPS: &str = "missing execution skips";
/// Text of the invalid skipped rule in this part.
pub(super) const INVALID_SKIPPED_RULE: &str = "invalid skipped rule";
/// Text of the unavailable execution capability contains as in this part.
pub(super) const UNAVAILABLE_EXECUTION_CAPABILITY_CONTAINS_AS: &str =
    "unavailable execution capability contains asserted accounting";
/// Text of the process failure has no complete report in this part.
pub(super) const PROCESS_FAILURE_HAS_NO_COMPLETE_REPORT: &str =
    "process failure has no complete-report execution accounting";
/// Text of the execution evidence does not satisfy the in this part.
pub(super) const EXECUTION_EVIDENCE_DOES_NOT_SATISFY_THE: &str =
    "execution evidence does not satisfy the declared contract";
/// Text of the orchestration in this part.
#[cfg(test)]
pub(super) const ORCHESTRATION: &str = "orchestration";
/// Text of the semantic authority txt in this part.
#[cfg(test)]
pub(super) const SEMANTIC_AUTHORITY_TXT: &str = "semantic-authority.txt";
/// Text of the f in this part.
#[cfg(test)]
pub(super) const F: &str = "f";
/// Text of the toy v1 in this part.
#[cfg(test)]
pub(super) const TOY_V1: &str = "toy-v1";
/// Text of the python3 10 in this part.
#[cfg(test)]
pub(super) const PYTHON3_10: &str = "python3.10";
/// Text of the toy fixture contract independent of process in this part.
#[cfg(test)]
pub(super) const TOY_FIXTURE_CONTRACT_INDEPENDENT_OF_PROCESS: &str =
    "toy fixture contract independent of process output";
/// Text of the no deadline in this part.
#[cfg(test)]
pub(super) const NO_DEADLINE: &str = "no-deadline";
/// Text of the no measured evidence in this part.
pub(super) const NO_MEASURED_EVIDENCE: &str = "no measured evidence";
/// Text of the diagnostic observations cannot publish bench in this part.
pub(super) const DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH_BENCH: &str =
    "diagnostic observations cannot publish benchmark metrics";
/// Text of the unknown or duplicate receipt registration ru in this part.
pub(super) const UNKNOWN_OR_DUPLICATE_RECEIPT_REGISTRATION_RU: &str =
    "unknown or duplicate receipt registration rule";
/// Text of the registered case name is missing in this part.
pub(super) const REGISTERED_CASE_NAME_IS_MISSING: &str = "registered case name is missing";
/// Text of the unknown or duplicate registered case in this part.
pub(super) const UNKNOWN_OR_DUPLICATE_REGISTERED_CASE: &str =
    "unknown or duplicate registered case";
/// Text of the receipt validation requires registered cases in this part.
pub(super) const RECEIPT_VALIDATION_REQUIRES_REGISTERED_CASES: &str =
    "receipt validation requires registered cases";
/// Text of the missing observed rule in this part.
pub(super) const MISSING_OBSERVED_RULE: &str = "missing observed rule";
/// Text of the missing observed case in this part.
pub(super) const MISSING_OBSERVED_CASE: &str = "missing observed case";
/// Text of the unregistered receipt identity in this part.
pub(super) const UNREGISTERED_RECEIPT_IDENTITY: &str = "unregistered receipt identity";
/// Text of the missing verification command in this part.
pub(super) const MISSING_VERIFICATION_COMMAND: &str = "missing verification command";
/// Text of the registered module inventory differs from pin in this part.
pub(super) const REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN: &str =
    "registered module inventory differs from pinned context profile";
/// Text of the missing declared module pin in this part.
pub(super) const MISSING_DECLARED_MODULE_PIN: &str = "missing declared module pin";
/// Text of the declared file observations require verificat in this part.
pub(super) const DECLARED_FILE_OBSERVATIONS_REQUIRE_VERIFICAT: &str =
    "declared file observations require verification preparation";
/// Text of the case selection is empty or outside in this part.
pub(super) const CASE_SELECTION_IS_EMPTY_OR_OUTSIDE: &str =
    "case selection is empty or outside this shard";
/// Text of the process failure verification requires a dire in this part.
pub(super) const PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE: &str =
    "process-failure verification requires a direct no-deadline observer";
/// Text of the reference returned a decoded report instead in this part.
pub(super) const REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD: &str =
    "reference returned a decoded report instead of the declared process failure";
