//! The native receipt keys and values the validation reads.
/// Text of the actual executable sha256 in this part.
pub(super) const ACTUAL_EXECUTABLE_SHA256: &str = "actual_executable_sha256";
/// Text of the approved executable sha256 in this part.
pub(super) const APPROVED_EXECUTABLE_SHA256: &str = "approved_executable_sha256";
/// Text of the availability in this part.
pub(super) const AVAILABILITY: &str = "availability";
/// Text of the basis in this part.
pub(super) const BASIS: &str = "basis";
/// Text of the benchmark case v2 in this part.
pub(super) const BENCHMARK_CASE_V2: &str = "benchmark-case/v2";
/// Candidate contestant label in differential failures.
pub(super) const CANDIDATE: &str = "candidate";
/// Text of the case passed in this part.
pub(super) const CASE_PASSED: &str = "case_passed";
/// Text of the complete in this part.
pub(super) const COMPLETE: &str = "complete";
/// Text of the context identity in this part.
pub(super) const CONTEXT_IDENTITY: &str = "context_identity";
/// Text of the context manifest in this part.
pub(super) const CONTEXT_MANIFEST: &str = "context_manifest";
/// Text of the coverage in this part.
pub(super) const COVERAGE: &str = "coverage";
/// Text of the diagnostics in this part.
pub(super) const DIAGNOSTICS: &str = "diagnostics";
/// Text of the discrepancy basis in this part.
pub(super) const DISCREPANCY_BASIS: &str = "discrepancy_basis";
/// Text of the discrepancy validation in this part.
pub(super) const DISCREPANCY_VALIDATION: &str = "discrepancy_validation";
/// Text of the documented reference discrepancy in this part.
pub(super) const DOCUMENTED_REFERENCE_DISCREPANCY: &str = "documented_reference_discrepancy";
/// Text of the documented tool contract difference in this part.
pub(super) const DOCUMENTED_TOOL_CONTRACT_DIFFERENCE: &str = "documented_tool_contract_difference";
/// Text of the end in this part.
pub(super) const END: &str = "end";
/// Text of the error in this part.
pub(super) const ERROR: &str = "error";
/// Text of the evaluated rule file pairs in this part.
pub(super) const EVALUATED_RULE_FILE_PAIRS: &str = "evaluated_rule_file_pairs";
/// Text of the execution metadata in this part.
pub(super) const EXECUTION_METADATA: &str = "execution_metadata";
/// Text of the expectation contract in this part.
pub(super) const EXPECTATION_CONTRACT: &str = "expectation_contract";
/// Text of the expected in this part.
pub(super) const EXPECTED: &str = "expected";
/// Text of the file in this part.
pub(super) const FILE: &str = "file";
/// Text of the files in this part.
pub(super) const FILES: &str = "files";
/// Text of the file metadata sha256 in this part.
pub(super) const FILE_METADATA_SHA256: &str = "file_metadata_sha256";
/// Text of the file metadata sha256 2 in this part.
pub(super) const FILE_METADATA_SHA256_2: &str = "file-metadata-sha256";
/// Text of the identity in this part.
pub(super) const IDENTITY: &str = "identity";
/// Text of the intended expected in this part.
pub(super) const INTENDED_EXPECTED: &str = "intended_expected";
/// Text of the intended output sha256 in this part.
pub(super) const INTENDED_OUTPUT_SHA256: &str = "intended_output_sha256";
/// Text of the intended output sha256 2 in this part.
pub(super) const INTENDED_OUTPUT_SHA256_2: &str = "intended-output-sha256";
/// Text of the isolated in this part.
pub(super) const ISOLATED: &str = "isolated";
/// Text of the language defect in this part.
pub(super) const LANGUAGE_DEFECT: &str = "language-defect";
/// Text of the logical root in this part.
pub(super) const LOGICAL_ROOT: &str = "logical_root";
/// Text of the manifest hash in this part.
pub(super) const MANIFEST_HASH: &str = "manifest_hash";
/// Text of the mode in this part.
pub(super) const MODE: &str = "mode";
/// Text of the name in this part.
pub(super) const NAME: &str = "name";
/// Text of the none in this part.
pub(super) const NONE: &str = "none";
/// Text of the observation schema in this part.
pub(super) const OBSERVATION_SCHEMA: &str = "observation_schema";
/// Text of the process in this part.
pub(super) const PROCESS: &str = "process";
/// Text of the process observation in this part.
pub(super) const PROCESS_OBSERVATION: &str = "process_observation";
/// Text of the profile sha256 in this part.
pub(super) const PROFILE_SHA256: &str = "profile_sha256";
/// Text of the profile sha256 2 in this part.
pub(super) const PROFILE_SHA256_2: &str = "profile-sha256";
/// Text of the project context in this part.
pub(super) const PROJECT_CONTEXT: &str = "project_context";
/// Text of the project context mode in this part.
pub(super) const PROJECT_CONTEXT_MODE: &str = "project_context_mode";
/// Text of the project inputs sha256 in this part.
pub(super) const PROJECT_INPUTS_SHA256: &str = "project_inputs_sha256";
/// Reference contestant label in differential failures.
pub(super) const REFERENCE: &str = "reference";
/// Text of the reference discrepancy in this part.
pub(super) const REFERENCE_DISCREPANCY: &str = "reference_discrepancy";
/// Text of the reference expected in this part.
pub(super) const REFERENCE_EXPECTED: &str = "reference_expected";
/// Text of the reference output sha256 in this part.
pub(super) const REFERENCE_OUTPUT_SHA256: &str = "reference_output_sha256";
/// Text of the reference output sha256 2 in this part.
pub(super) const REFERENCE_OUTPUT_SHA256_2: &str = "reference-output-sha256";
/// Text of the rule in this part.
pub(super) const RULE: &str = "rule";
/// Text of the rule context in this part.
pub(super) const RULE_CONTEXT: &str = "rule_context";
/// Text of the rule observations in this part.
pub(super) const RULE_OBSERVATIONS: &str = "rule_observations";
/// Text of the selected rules in this part.
pub(super) const SELECTED_RULES: &str = "selected_rules";
/// Text of the selected rule file pairs in this part.
pub(super) const SELECTED_RULE_FILE_PAIRS: &str = "selected_rule_file_pairs";
/// Text of the settings sha256 in this part.
pub(super) const SETTINGS_SHA256: &str = "settings_sha256";
/// Text of the settings sha256 2 in this part.
pub(super) const SETTINGS_SHA256_2: &str = "settings-sha256";
/// Text of the side in this part.
pub(super) const SIDE: &str = "side";
/// Text of the skipped in this part.
pub(super) const SKIPPED: &str = "skipped";
/// Text of the skipped rule file pairs in this part.
pub(super) const SKIPPED_RULE_FILE_PAIRS: &str = "skipped_rule_file_pairs";
/// Text of the source sha256 in this part.
pub(super) const SOURCE_SHA256: &str = "source_sha256";
/// Text of the source sha256 2 in this part.
pub(super) const SOURCE_SHA256_2: &str = "source-sha256";
/// Text of the start in this part.
pub(super) const START: &str = "start";
/// Text of the tool contract difference in this part.
pub(super) const TOOL_CONTRACT_DIFFERENCE: &str = "tool-contract-difference";
/// Text of the unknown or unread rule file pairs in this part.
pub(super) const UNKNOWN_OR_UNREAD_RULE_FILE_PAIRS: &str = "unknown_or_unread_rule_file_pairs";
