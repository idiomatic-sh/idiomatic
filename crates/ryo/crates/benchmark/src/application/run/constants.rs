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
/// Text of the duplicate reference expectation identity in this part.
pub(super) const DUPLICATE_REFERENCE_EXPECTATION_IDENTITY: &str =
    "duplicate reference expectation identity";
/// Text of the reference expectation outside executable cas in this part.
pub(super) const REFERENCE_EXPECTATION_OUTSIDE_EXECUTABLE_CAS: &str =
    "reference expectation outside executable case inventory";
/// Text of the duplicate canonical reference expectation in this part.
pub(super) const DUPLICATE_CANONICAL_REFERENCE_EXPECTATION: &str =
    "duplicate canonical reference expectation";
/// Text of the discrepancy requires lowercase sha256 identi in this part.
pub(super) const DISCREPANCY_REQUIRES_LOWERCASE_SHA256_IDENTI: &str =
    "discrepancy requires lowercase SHA256 identities";
/// Text of the process expectation cannot also declare refe in this part.
pub(super) const PROCESS_EXPECTATION_CANNOT_ALSO_DECLARE_REFE: &str =
    "process expectation cannot also declare reference diagnostics";
/// Text of the invalid reference expectation source range in this part.
pub(super) const INVALID_REFERENCE_EXPECTATION_SOURCE_RANGE: &str =
    "invalid reference expectation source/range";
/// Text of the tool contract differences require complete d in this part.
pub(super) const TOOL_CONTRACT_DIFFERENCES_REQUIRE_COMPLETE_D: &str =
    "tool-contract differences require complete diagnostic outputs";
/// Text of the independent evidence requires explicit langu in this part.
pub(super) const INDEPENDENT_EVIDENCE_REQUIRES_EXPLICIT_LANGU: &str =
    "independent evidence requires explicit language contract and version rationale";
/// Text of the tool contract evidence needs a pinned in this part.
pub(super) const TOOL_CONTRACT_EVIDENCE_NEEDS_A_PINNED: &str =
    "tool contract evidence needs a pinned public source revision and policy reason";
/// Text of the prior worker evidence needs a named in this part.
pub(super) const PRIOR_WORKER_EVIDENCE_NEEDS_A_NAMED: &str =
    "prior worker evidence needs a named control and policy reason";
/// Text of the discrepancy evidence does not match its in this part.
pub(super) const DISCREPANCY_EVIDENCE_DOES_NOT_MATCH_ITS: &str =
    "discrepancy evidence does not match its declared authority";
/// Text of the tool contract difference requires one public in this part.
pub(super) const TOOL_CONTRACT_DIFFERENCE_REQUIRES_ONE_PUBLIC: &str =
    "tool-contract difference requires one public contract and one prior worker observation";
/// Text of the process expectation is not a diagnostic in this part.
pub(super) const PROCESS_EXPECTATION_IS_NOT_A_DIAGNOSTIC: &str =
    "process expectation is not a diagnostic report";
/// Text of the file in this part.
pub(super) const FILE: &str = "file";
/// Text of the start in this part.
pub(super) const START: &str = "start";
/// Text of the end in this part.
pub(super) const END: &str = "end";
/// Text of the reference discrepancy requires complete know in this part.
pub(super) const REFERENCE_DISCREPANCY_REQUIRES_COMPLETE_KNOW: &str =
    "reference discrepancy requires complete, known execution coverage";
/// Text of the reference discrepancy requires distinct full in this part.
pub(super) const REFERENCE_DISCREPANCY_REQUIRES_DISTINCT_FULL: &str =
    "reference discrepancy requires distinct full expectations and independent evidence";
/// Text of the missing profile root in this part.
pub(super) const MISSING_PROFILE_ROOT: &str = "missing profile root";
/// Text of the independent evidence does not support config in this part.
pub(super) const INDEPENDENT_EVIDENCE_DOES_NOT_SUPPORT_CONFIG: &str =
    "independent evidence does not support configured language contract";
/// Text of the discrepancy authority and evidence differ in this part.
pub(super) const DISCREPANCY_AUTHORITY_AND_EVIDENCE_DIFFER: &str =
    "discrepancy authority and evidence differ";
/// Text of the stale reference discrepancy settings source in this part.
pub(super) const STALE_REFERENCE_DISCREPANCY_SETTINGS_SOURCE: &str =
    "stale reference discrepancy settings/source/expectation identity";
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
/// Text of the stale reference discrepancy tool profile or in this part.
pub(super) const STALE_REFERENCE_DISCREPANCY_TOOL_PROFILE_OR: &str =
    "stale reference discrepancy tool/profile or unapproved executable";
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
/// Text of the project search roots in this part.
pub(super) const PROJECT_SEARCH_ROOTS: &str = "project_search_roots";
/// Text of the given project inventory in this part.
pub(super) const GIVEN_PROJECT_INVENTORY: &str = "given_project_inventory";
/// Text of the project context mode in this part.
pub(super) const PROJECT_CONTEXT_MODE: &str = "project_context_mode";
/// Text of the non utf8 reference invocation in this part.
pub(super) const NON_UTF8_REFERENCE_INVOCATION: &str = "non-UTF8 reference invocation";
/// Text of the state 2 in this part.
pub(super) const STATE_2: &str = "$STATE";
/// Text of the root in this part.
pub(super) const ROOT: &str = "$ROOT";
/// Text of the slash in this part.
pub(super) const SLASH: &str = "/";
/// Text of the parser configuration in this part.
pub(super) const PARSER_CONFIGURATION: &str = "parser_configuration";
/// Text of the performance in this part.
pub(super) const PERFORMANCE: &str = "performance";
/// Text of the rule context declaration in this part.
pub(super) const RULE_CONTEXT_DECLARATION: &str = "rule_context_declaration";
/// Text of the source language in this part.
pub(super) const SOURCE_LANGUAGE: &str = "source_language";
/// Text of the missing normalized invocation configuration in this part.
pub(super) const MISSING_NORMALIZED_INVOCATION_CONFIGURATION: &str =
    "missing normalized invocation configuration";
/// Text of the raw invocation root is missing in this part.
pub(super) const RAW_INVOCATION_ROOT_IS_MISSING: &str = "raw invocation root is missing";
/// Text of the raw invocation root is not absolute in this part.
pub(super) const RAW_INVOCATION_ROOT_IS_NOT_ABSOLUTE: &str = "raw invocation root is not absolute";
/// Text of the raw invocation state is missing in this part.
pub(super) const RAW_INVOCATION_STATE_IS_MISSING: &str = "raw invocation state is missing";
/// Text of the nonutf8 invocation value in this part.
pub(super) const NONUTF8_INVOCATION_VALUE: &str = "nonUTF8 invocation value";
/// Text of the raw argv is missing in this part.
pub(super) const RAW_ARGV_IS_MISSING: &str = "raw argv is missing";
/// Text of the raw environment is missing in this part.
pub(super) const RAW_ENVIRONMENT_IS_MISSING: &str = "raw environment is missing";
/// Text of the process failure requires a positive numeric in this part.
pub(super) const PROCESS_FAILURE_REQUIRES_A_POSITIVE_NUMERIC: &str =
    "process failure requires a positive numeric exit";
/// Text of the literal in this part.
pub(super) const LITERAL: &str = "literal";
/// Text of the value in this part.
pub(super) const VALUE: &str = "value";
/// Text of the input path in this part.
pub(super) const INPUT_PATH: &str = "input-path";
/// Text of the encoding in this part.
pub(super) const ENCODING: &str = "encoding";
/// Text of the utf8 bytes in this part.
pub(super) const UTF8_BYTES: &str = "utf8-bytes";
/// Text of the json string content in this part.
pub(super) const JSON_STRING_CONTENT: &str = "json-string-content";
/// Text of the stdout in this part.
pub(super) const STDOUT: &str = "stdout";
/// Text of the stderr in this part.
pub(super) const STDERR: &str = "stderr";
/// Text of the no process expectation in this part.
pub(super) const NO_PROCESS_EXPECTATION: &str = "no process expectation";
/// Text of the actual process identity is not the in this part.
pub(super) const ACTUAL_PROCESS_IDENTITY_IS_NOT_THE: &str =
    "actual process identity is not the validated exact observation group";
/// Text of the reference termination differs from declared in this part.
pub(super) const REFERENCE_TERMINATION_DIFFERS_FROM_DECLARED: &str =
    "reference termination differs from declared completed failure";
/// Text of the reference process bytes differ from declared in this part.
pub(super) const REFERENCE_PROCESS_BYTES_DIFFER_FROM_DECLARED: &str =
    "reference process bytes differ from declared expectation";
/// Text of the missing process profile boundary in this part.
pub(super) const MISSING_PROCESS_PROFILE_BOUNDARY: &str = "missing process profile boundary";
/// Text of the missing validated source identity in this part.
pub(super) const MISSING_VALIDATED_SOURCE_IDENTITY: &str = "missing validated source identity";
/// Text of the missing process contract in this part.
pub(super) const MISSING_PROCESS_CONTRACT: &str = "missing process contract";
/// Text of the process evidence has an unapproved exact in this part.
pub(super) const PROCESS_EVIDENCE_HAS_AN_UNAPPROVED_EXACT: &str =
    "process evidence has an unapproved exact binary";
/// Text of the process observations must retain separate ra in this part.
pub(super) const PROCESS_OBSERVATIONS_MUST_RETAIN_SEPARATE_RA: &str =
    "process observations must retain separate raw artifacts";
/// Text of the process expectation needs two captures for in this part.
pub(super) const PROCESS_EXPECTATION_NEEDS_TWO_CAPTURES_FOR: &str =
    "process expectation needs two captures for the actual exact executable";
/// Text of the missing raw invocation in this part.
pub(super) const MISSING_RAW_INVOCATION: &str = "missing raw invocation";
/// Text of the archived request root is missing in this part.
pub(super) const ARCHIVED_REQUEST_ROOT_IS_MISSING: &str = "archived request root is missing";
/// Text of the archived process bytes termination or immuta in this part.
pub(super) const ARCHIVED_PROCESS_BYTES_TERMINATION_OR_IMMUTA: &str =
    "archived process bytes, termination or immutable identity differ";
/// Text of the archived state boundary is missing in this part.
pub(super) const ARCHIVED_STATE_BOUNDARY_IS_MISSING: &str = "archived state boundary is missing";
/// Text of the archived invocation directory is missing in this part.
pub(super) const ARCHIVED_INVOCATION_DIRECTORY_IS_MISSING: &str =
    "archived invocation directory is missing";
/// Text of the archived argv environment do not establish in this part.
pub(super) const ARCHIVED_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH: &str =
    "archived argv/environment do not establish the pinned harness settings";
/// Text of the process observation groups must retain disti in this part.
pub(super) const PROCESS_OBSERVATION_GROUPS_MUST_RETAIN_DISTI: &str =
    "process observation groups must retain distinct raw captures";
/// Text of the missing reproducible exact binary observatio in this part.
pub(super) const MISSING_REPRODUCIBLE_EXACT_BINARY_OBSERVATIO: &str =
    "missing reproducible exact-binary observation group";
/// Text of the invocation in this part.
pub(super) const INVOCATION: &str = "invocation";
/// Text of the sha256 in this part.
pub(super) const SHA256: &str = "sha256";
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
/// Text of the missing captured process bytes in this part.
pub(super) const MISSING_CAPTURED_PROCESS_BYTES: &str = "missing captured process bytes";
/// Text of the captured process byte digest differs in this part.
pub(super) const CAPTURED_PROCESS_BYTE_DIGEST_DIFFERS: &str =
    "captured process byte digest differs";
/// Text of the missing actual process executable in this part.
pub(super) const MISSING_ACTUAL_PROCESS_EXECUTABLE: &str = "missing actual process executable";
/// Text of the reference process variant or exact terminati in this part.
pub(super) const REFERENCE_PROCESS_VARIANT_OR_EXACT_TERMINATI: &str =
    "reference process variant or exact termination identity differs";
/// Text of the process certificate file metadata identity d in this part.
pub(super) const PROCESS_CERTIFICATE_FILE_METADATA_IDENTITY_D: &str =
    "process certificate file metadata identity differs";
/// Text of the process receipt request root is missing in this part.
pub(super) const PROCESS_RECEIPT_REQUEST_ROOT_IS_MISSING: &str =
    "process receipt request root is missing";
/// Text of the process observation root differs from receip in this part.
pub(super) const PROCESS_OBSERVATION_ROOT_DIFFERS_FROM_RECEIP: &str =
    "process observation root differs from receipt";
/// Text of the missing registered rule in this part.
pub(super) const MISSING_REGISTERED_RULE: &str = "missing registered rule";
/// Text of the missing registered case in this part.
pub(super) const MISSING_REGISTERED_CASE: &str = "missing registered case";
/// Text of the missing raw process path in this part.
pub(super) const MISSING_RAW_PROCESS_PATH: &str = "missing raw process path";
/// Text of the raw process artifact belongs to another in this part.
pub(super) const RAW_PROCESS_ARTIFACT_BELONGS_TO_ANOTHER: &str =
    "raw process artifact belongs to another case identity";
/// Text of the captured process bytes differ from registere in this part.
pub(super) const CAPTURED_PROCESS_BYTES_DIFFER_FROM_REGISTERE: &str =
    "captured process bytes differ from registered templates";
/// Text of the process metadata capture failed in this part.
pub(super) const PROCESS_METADATA_CAPTURE_FAILED: &str = "process metadata capture failed";
/// Text of the raw process invocation differs at file in this part.
pub(super) const RAW_PROCESS_INVOCATION_DIFFERS_AT_FILE: &str =
    "raw process invocation differs at file_metadata_sha256";
/// Text of the missing pinned normalized invocation in this part.
pub(super) const MISSING_PINNED_NORMALIZED_INVOCATION: &str =
    "missing pinned normalized invocation";
/// Text of the raw argv environment do not establish in this part.
pub(super) const RAW_ARGV_ENVIRONMENT_DO_NOT_ESTABLISH: &str =
    "raw argv/environment do not establish pinned settings";
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
