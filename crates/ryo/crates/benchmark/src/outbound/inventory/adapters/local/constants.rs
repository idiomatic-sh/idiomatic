// ignore: idiomatic.arch/file-length - the one constants file of the local inventory adapter: the registration, case, discrepancy, authored-case, profile and instance documents name their shared fields once here, and the adapter spec places no second constants file
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
/// The newline that ends a summary file.
pub(super) const NEWLINE: &[u8] = b"\n";

/// Text `rule-applicability`.
pub(super) const APPLICABILITY_KEY: &str = "rule-applicability";
/// Text `approved_executable_sha256`.
pub(super) const APPROVED: &str = "approved_executable_sha256";
/// Text `auxiliary-files`.
pub(super) const AUXILIARY_FILES: &str = "auxiliary-files";
/// Text `basis`.
pub(super) const BASIS: &str = "basis";
/// Text `cases`.
pub(super) const CASES: &str = "cases";
/// Text `cases_file`.
pub(super) const CASES_FILE_KEY: &str = "cases_file";
/// Text `cases_sha256`.
pub(super) const CASES_SHA256: &str = "cases_sha256";
/// Text `case_count`.
pub(super) const CASE_COUNT: &str = "case_count";
/// Text `case files must map names to source text`.
pub(super) const CASE_FILES: &str = "case files must map names to source text";
/// The refusal: staged case inventory is empty or has duplicate (rule, name) identities.
pub(super) const CASE_IDENTITIES: &str =
    "staged case inventory is empty or has duplicate (rule, name) identities";
/// Text `every case needs a name`.
pub(super) const CASE_NAME: &str = "every case needs a name";
/// Text `comparison`.
pub(super) const COMPARISON: &str = "comparison";
/// Text `completeness`.
pub(super) const COMPLETENESS: &str = "completeness";
/// The refusal of a complete Given inventory without declared roots.
pub(super) const COMPLETE_NEEDS_ROOTS: &str =
    "a complete Given inventory needs declared project search roots";
/// Text `complete-report`.
pub(super) const CONTRACT_COMPLETE_REPORT: &str = "complete-report";
/// Text `complete-within-declared-scope`.
pub(super) const COMPLETE_SCOPE: &str = "complete-within-declared-scope";
/// Text `configuration`.
pub(super) const CONFIGURATION: &str = "configuration";
/// Text `context`.
pub(super) const CONTEXT: &str = "context";
/// Text `unsupported project context mode`.
pub(super) const CONTEXT_MODE: &str = "unsupported project context mode";
/// Text `context source, auxiliary and missing roles overlap`.
pub(super) const CONTEXT_ROLES_OVERLAP: &str =
    "context source, auxiliary and missing roles overlap";
/// The refusal: tool contract difference needs public contract and prior worker evidence.
pub(super) const CONTRACT_EVIDENCE_PAIR: &str =
    "tool contract difference needs public contract and prior worker evidence";
/// The fields of public tool contract evidence.
pub(super) const CONTRACT_FIELDS: [&str; 6] = [
    "path",
    "sha256",
    "kind",
    "source-revision",
    "source-path",
    "rationale",
];
/// Text `.`.
pub(super) const CURRENT_DIRECTORY: &str = ".";
/// Text `defaults`.
pub(super) const DEFAULTS_KEY: &str = "defaults";
/// Text `deferred`.
pub(super) const DEFERRED_KEY: &str = "deferred";
/// Text `diagnostics`.
pub(super) const DIAGNOSTICS_CONTRACT: &str = "diagnostics";
/// Text `diagnostics`.
pub(super) const DIAGNOSTICS_KEY: &str = "diagnostics";
/// The worker's diagnostics, every one off in a registered run.
pub(super) const DIAGNOSTICS_OFF: [&str; 8] = [
    "execution",
    "resource_routes",
    "row_progress",
    "steps",
    "joins",
    "prefixes",
    "subplans",
    "selection_detail",
];
/// Text `reference discrepancy requires lowercase SHA256 identities`.
pub(super) const DISCREPANCY_DIGESTS: &str =
    "reference discrepancy requires lowercase SHA256 identities";
/// Text `reference discrepancy requires full comparison and a reason`.
pub(super) const DISCREPANCY_FULL: &str =
    "reference discrepancy requires full comparison and a reason";
/// Text `incomplete reference discrepancy identity`.
pub(super) const DISCREPANCY_IDENTITY: &str = "incomplete reference discrepancy identity";
/// Text `discrepancy diagnostic outside selected source`.
pub(super) const DISCREPANCY_OUTSIDE: &str = "discrepancy diagnostic outside selected source";
/// Text `invalid discrepancy diagnostic range`.
pub(super) const DISCREPANCY_RANGE: &str = "invalid discrepancy diagnostic range";
/// The refusal: full discrepancy expectations require explicit diagnostic identities.
pub(super) const DISCREPANCY_ROWS: &str =
    "full discrepancy expectations require explicit diagnostic identities";
/// Text `invalid reference-discrepancy schema`.
pub(super) const DISCREPANCY_SCHEMA: &str = "invalid reference-discrepancy schema";
/// Text `effective_settings`.
pub(super) const EFFECTIVE_SETTINGS: &str = "effective_settings";
/// Text `end`.
pub(super) const END: &str = "end";
/// Text `entries`.
pub(super) const ENTRIES: &str = "entries";
/// The kinds a Given project entry may have.
pub(super) const ENTRY_STATES: [&str; 5] = [
    STATE_FILE,
    STATE_DIRECTORY,
    STATE_OTHER,
    STATE_MISSING,
    STATE_UNKNOWN,
];
/// Text `file`: a regular file entry.
pub(super) const STATE_FILE: &str = "file";
/// Text `directory`.
pub(super) const STATE_DIRECTORY: &str = "directory";
/// Text `other`.
pub(super) const STATE_OTHER: &str = "other";
/// Text `missing`.
pub(super) const STATE_MISSING: &str = "missing";
/// Text `unknown`.
pub(super) const STATE_UNKNOWN: &str = "unknown";
/// Text `evidence`.
pub(super) const EVIDENCE: &str = "evidence";
/// Text `evidence does not support the declared contract`.
pub(super) const EVIDENCE_CONTRACT: &str = "evidence does not support the declared contract";
/// Text `incomplete discrepancy evidence`.
pub(super) const EVIDENCE_INCOMPLETE: &str = "incomplete discrepancy evidence";
/// Text `unsupported tool contract evidence kind`.
pub(super) const EVIDENCE_KIND: &str = "unsupported tool contract evidence kind";
/// Text `reference discrepancy requires independent evidence`.
pub(super) const EVIDENCE_REQUIRED: &str = "reference discrepancy requires independent evidence";
/// Text `evidence_source_root`.
pub(super) const EVIDENCE_SOURCE_ROOT: &str = "evidence_source_root";
/// Text `expected`.
pub(super) const EXPECTED_KEY: &str = "expected";
/// Text `an expected finding names its file, start and end`.
pub(super) const EXPECTED_ROW: &str = "an expected finding names its file, start and end";
/// Text `file`.
pub(super) const FILE: &str = "file";
/// Text `files`.
pub(super) const FILES: &str = "files";
/// Text `files`.
pub(super) const FILES_KEY: &str = "files";
/// Text `file-metadata`.
pub(super) const FILE_METADATA: &str = "file-metadata";
/// Text `file metadata must map declared paths to booleans`.
pub(super) const FILE_METADATA_FLAGS: &str = "file metadata must map declared paths to booleans";
/// Text `file-metadata-sha256`.
pub(super) const FILE_METADATA_SHA256_KEY: &str = "file-metadata-sha256";
/// Text `file-observation`.
pub(super) const FILE_OBSERVATION: &str = "file-observation";
/// The refusal of an unsupported file observation.
pub(super) const FILE_OBSERVATION_REQUIREMENT: &str = "unsupported file observation requirement";
/// Text `full`.
pub(super) const FULL: &str = "full";
/// Text `Given project entry has an invalid path or kind`.
pub(super) const GIVEN_ENTRY: &str = "Given project entry has an invalid path or kind";
/// Text `Given project inventory has an invalid shape`.
pub(super) const GIVEN_INVENTORY_SHAPE: &str = "Given project inventory has an invalid shape";
/// Text `given-project-inventory`.
pub(super) const GIVEN_PROJECT_INVENTORY: &str = "given-project-inventory";
/// Text `identical expectations are not a reference discrepancy`.
pub(super) const IDENTICAL_EXPECTATIONS: &str =
    "identical expectations are not a reference discrepancy";
/// Text `identity`.
pub(super) const IDENTITY: &str = "identity";
/// The identity every documented difference names.
pub(super) const IDENTITY_FIELDS: [&str; 8] = [
    "tool",
    "version",
    "observed-executable-sha256",
    "profile-sha256",
    "settings-sha256",
    "source-sha256",
    "reference-output-sha256",
    "intended-output-sha256",
];
/// Text `independent-language-run`.
pub(super) const INDEPENDENT_RUN: &str = "independent-language-run";
/// Text `intended_expected`.
pub(super) const INTENDED_EXPECTED: &str = "intended_expected";
/// Text `intended-output-sha256`.
pub(super) const INTENDED_OUTPUT: &str = "intended-output-sha256";
/// Text `isolated`.
pub(super) const ISOLATED: &str = "isolated";
/// Text `isolated profile cannot admit a context-authored case`.
pub(super) const ISOLATED_CONTEXT: &str = "isolated profile cannot admit a context-authored case";
/// Text `kind`.
pub(super) const KIND: &str = "kind";
/// Text `language-defect`.
pub(super) const LANGUAGE_DEFECT: &str = "language-defect";
/// The fields of language evidence.
pub(super) const LANGUAGE_FIELDS: [&str; 6] = [
    "path",
    "sha256",
    "kind",
    "language-version",
    "target-configuration",
    "version-rationale",
];
/// Text `language-spec`.
pub(super) const LANGUAGE_SPEC: &str = "language-spec";
/// Text `language-version`.
pub(super) const LANGUAGE_VERSION: &str = "language-version";
/// The refusal: listed-disk profile requires an explicit context for every case.
pub(super) const LISTED_CONTEXT: &str =
    "listed-disk profile requires an explicit context for every case";
/// Text `listed-disk`.
pub(super) const LISTED_DISK: &str = "listed-disk";
/// Text `logical-root`.
pub(super) const LOGICAL_ROOT: &str = "logical-root";
/// Text `logical_root`.
pub(super) const LOGICAL_ROOT_KEY: &str = "logical_root";
/// Text `manifest_hash`.
pub(super) const MANIFEST_HASH: &str = "manifest_hash";
/// Text `missing`.
pub(super) const MISSING: &str = "missing";
/// Text `missing_paths`.
pub(super) const MISSING_PATHS: &str = "missing_paths";
/// Text `mode`.
pub(super) const MODE: &str = "mode";
/// Text `module_sha256`.
pub(super) const MODULE_SHA256: &str = "module_sha256";
/// Text `name`.
pub(super) const NAME_KEY: &str = "name";
/// The refusal of a rule without a declared applicability.
pub(super) const NO_APPLICABILITY: &str = "no declared applicability for a registered rule";
/// Text `observed-executable-sha256`.
pub(super) const OBSERVED_EXECUTABLE: &str = "observed-executable-sha256";
/// Text `output`.
pub(super) const OUTPUT: &str = "output";
/// Text `parser-configuration`.
pub(super) const PARSER_CONFIGURATION: &str = "parser-configuration";
/// Text `parser_configuration`.
pub(super) const PARSER_CONFIGURATION_KEY: &str = "parser_configuration";
/// Text `partial`.
pub(super) const PARTIAL: &str = "partial";
/// Text `path`.
pub(super) const PATH: &str = "path";
/// Text `pending`.
pub(super) const PENDING_KEY: &str = "pending";
/// The one file observation a profile may require.
pub(super) const POSIX_EXECUTABLE: &str = "posix-executable";
/// Text `present_paths`.
pub(super) const PRESENT_PATHS: &str = "present_paths";
/// The digests prior worker evidence names.
pub(super) const PRIOR_DIGESTS: [&str; 4] = [
    "worker-sha256",
    "control-source-sha256",
    "control-query-sha256",
    "control-profile-sha256",
];
/// The fields of prior worker evidence.
pub(super) const PRIOR_FIELDS: [&str; 9] = [
    "path",
    "sha256",
    "kind",
    "worker-sha256",
    "control-source-sha256",
    "control-query-sha256",
    "control-profile-sha256",
    "witness-case",
    "rationale",
];
/// Text `prior-worker-run`.
pub(super) const PRIOR_WORKER: &str = "prior-worker-run";
/// Text `process`.
pub(super) const PROCESS: &str = "process";
/// The refusal: process discrepancy requires an explicit full-comparison wire contract.
pub(super) const PROCESS_CONTRACT: &str =
    "process discrepancy requires an explicit full-comparison wire contract";
/// Text `process-failure`.
pub(super) const PROCESS_FAILURE_KIND: &str = "process-failure";
/// The refusal: full process discrepancy requires explicit intended diagnostic identities.
pub(super) const PROCESS_FINDINGS: &str =
    "full process discrepancy requires explicit intended diagnostic identities";
/// Text `unknown process expectation wire variant`.
pub(super) const PROCESS_VARIANT: &str = "unknown process expectation wire variant";
/// Text `producers`.
pub(super) const PRODUCERS: &str = "producers";
/// Text `profile-sha256`.
pub(super) const PROFILE_SHA256: &str = "profile-sha256";
/// Text `profile_sha256`.
pub(super) const PROFILE_SHA256_KEY: &str = "profile_sha256";
/// Text `project-context-mode`.
pub(super) const PROJECT_CONTEXT_MODE: &str = "project-context-mode";
/// Text `project-search-roots`.
pub(super) const PROJECT_SEARCH_ROOTS: &str = "project-search-roots";
/// Text `public-tool-contract`.
pub(super) const PUBLIC_CONTRACT: &str = "public-tool-contract";
/// Text `query_file`.
pub(super) const QUERY_FILE: &str = "query_file";
/// Text `query-modules`.
pub(super) const QUERY_MODULES: &str = "query-modules";
/// Text `query_sha256`.
pub(super) const QUERY_SHA256: &str = "query_sha256";
/// Text `rationale`.
pub(super) const RATIONALE: &str = "rationale";
/// Text `reason`.
pub(super) const REASON: &str = "reason";
/// Text `reference-discrepancy`.
pub(super) const REFERENCE_DISCREPANCY: &str = "reference-discrepancy";
/// Text `reference_expected`.
pub(super) const REFERENCE_EXPECTED: &str = "reference_expected";
/// Text `reference-output-sha256`.
pub(super) const REFERENCE_OUTPUT: &str = "reference-output-sha256";
/// Text `requested-files`.
pub(super) const REQUESTED_FILES: &str = "requested-files";
/// The keys a staged profile may hold: every reviewed execution setting.
pub(super) const REVIEWED_KEYS: [&str; 16] = [
    "rule-observation-selection",
    "supporting-rules",
    "project-search-roots",
    "parser-configuration",
    "deferred",
    "file-observation",
    "project-context-mode",
    "sha256",
    "tool",
    "defaults",
    "query-modules",
    "rule-observations",
    "version",
    "source-language",
    "rule-applicability",
    "rule-context",
];
/// The length of a hexadecimal commit revision.
pub(super) const REVISION_HEX_LENGTH: usize = 40;
/// Text `rule`.
pub(super) const RULE: &str = "rule";
/// Text `rules`.
pub(super) const RULES_DIRECTORY: &str = "rules";
/// Text `rule-context`.
pub(super) const RULE_CONTEXT: &str = "rule-context";
/// Text `invalid rule context declaration`.
pub(super) const RULE_CONTEXT_DECLARATION: &str = "invalid rule context declaration";
/// The refusal: rule context override requires an explicit profile declaration.
pub(super) const RULE_CONTEXT_OVERRIDE: &str =
    "rule context override requires an explicit profile declaration";
/// Text `rule-observations`.
pub(super) const RULE_OBSERVATIONS_FIELD: &str = "rule-observations";
/// Text `rule-observation-selection`.
pub(super) const RULE_OBSERVATION_SELECTION: &str = "rule-observation-selection";
/// Text `project search roots must be ordered safe relative paths`.
pub(super) const SEARCH_ROOTS: &str = "project search roots must be ordered safe relative paths";
/// Text `selected`.
pub(super) const SELECTED: &str = "selected";
/// Text `invalid selected discrepancy files`.
pub(super) const SELECTED_FILES: &str = "invalid selected discrepancy files";
/// Text `selected-rules`.
pub(super) const SELECTED_RULES: &str = "selected-rules";
/// Text `settings_source`.
pub(super) const SETTINGS_SOURCE: &str = "settings_source";
/// Text `settings_source_sha256`.
pub(super) const SETTINGS_SOURCE_SHA256: &str = "settings_source_sha256";
/// Text `sha256`.
pub(super) const SHA256_KEY: &str = "sha256";
/// Text `skipped`.
pub(super) const SKIPPED_KEY: &str = "skipped";
/// Text `source-language`.
pub(super) const SOURCE_LANGUAGE: &str = "source-language";
/// The refusal of a source language that is not an exact nonblank provider name.
pub(super) const SOURCE_LANGUAGE_NAME: &str =
    "source language must be an exact nonblank provider name";
/// Text `source-path`.
pub(super) const SOURCE_PATH: &str = "source-path";
/// Text `source-revision`.
pub(super) const SOURCE_REVISION: &str = "source-revision";
/// Text `source-sha256`.
pub(super) const SOURCE_SHA256: &str = "source-sha256";
/// Text `stale/external evidence or unsupported language contract`.
pub(super) const STALE_EVIDENCE: &str = "stale/external evidence or unsupported language contract";
/// Text `stale reference/intended expectation identity`.
pub(super) const STALE_EXPECTATIONS: &str = "stale reference/intended expectation identity";
/// Text `stale discrepancy file metadata identity`.
pub(super) const STALE_METADATA: &str = "stale discrepancy file metadata identity";
/// Text `stale discrepancy source identity`.
pub(super) const STALE_SOURCE: &str = "stale discrepancy source identity";
/// Text `stale discrepancy tool/profile identity`.
pub(super) const STALE_TOOL_PROFILE: &str = "stale discrepancy tool/profile identity";
/// Text `start`.
pub(super) const START: &str = "start";
/// Text `state`.
pub(super) const STATE: &str = "state";
/// Text `supporting-rules`.
pub(super) const SUPPORTING_KEY: &str = "supporting-rules";
/// Text `tables`.
pub(super) const TABLES: &str = "tables";
/// Text `target-configuration`.
pub(super) const TARGET_CONFIGURATION: &str = "target-configuration";
/// Text `tool`.
pub(super) const TOOL: &str = "tool";
/// Text `tool-contract-difference`.
pub(super) const TOOL_CONTRACT: &str = "tool-contract-difference";
/// Text `tool-contract-difference`.
pub(super) const TOOL_CONTRACT_DIFFERENCE: &str = "tool-contract-difference";
/// Text `a case cannot declare two discrepancy authorities`.
pub(super) const TWO_AUTHORITIES: &str = "a case cannot declare two discrepancy authorities";
/// The refusal of a staged profile with an unreviewed execution setting.
pub(super) const UNREVIEWED_SETTINGS: &str = "unreviewed execution settings in canonical profile";
/// Text `unsafe context path`.
pub(super) const UNSAFE_CONTEXT: &str = "unsafe context path";
/// Text `unsafe discrepancy evidence path`.
pub(super) const UNSAFE_EVIDENCE: &str = "unsafe discrepancy evidence path";
/// The refusal: a profile or case holds a value the registration cannot represent.
pub(super) const UNSUPPORTED_DOCUMENT_VALUE: &str =
    "a profile or case holds a value the registration cannot represent";
/// Text `version`.
pub(super) const VERSION: &str = "version";
/// Text `version-rationale`.
pub(super) const VERSION_RATIONALE: &str = "version-rationale";
/// Text `witness-case`.
pub(super) const WITNESS_CASE: &str = "witness-case";
