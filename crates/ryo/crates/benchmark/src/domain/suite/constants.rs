//! Stable operation vocabulary and artifact identities.
/// An empty inventory cannot establish default coverage.
pub(super) const EMPTY_AUDIT: &str = "empty audit is not default coverage";
/// A benchmark must execute at least one authored default rule.
pub(crate) const EMPTY_SUBSET: &str = "no authored default subset";
/// Readable separator for missing or extra rule identities.
pub(super) const RULE_SEPARATOR: &str = ", ";
/// Every authored rule needs both positive and negative evidence.
pub(super) const MISSING_POLARITY: &str = "a rule requires positive and negative cases";
/// An empty fixture file provides no evidence.
pub(super) const EMPTY_CASES: &str = "case evidence must be nonempty";
/// Refuse duplicate case names or cases without sources.
pub(super) const INVALID_CASE_IDENTITY: &str = "case names must be unique and sources nonempty";
/// Expected findings must refer to an included source.
pub(super) const MISSING_SOURCE: &str = "expected finding outside case";
/// Expected offsets must be ordered UTF-8 boundaries inside the source.
pub(super) const INVALID_SOURCE_RANGE: &str = "expected finding has invalid UTF-8 span";
/// Only distinct authored default rules may enter a selected profile.
pub(super) const INVALID_SELECTION: &str =
    "selected subset must contain distinct authored default rules";

/// Full-report expectations must identify every diagnostic independently.
pub(super) const MISSING_DIAGNOSTIC_IDENTITY: &str =
    "full-report expectations require explicit rule identities";
/// Text of the metadata names an undeclared source in this part.
pub(super) const METADATA_NAMES_AN_UNDECLARED_SOURCE: &str = "metadata names an undeclared source";
/// Text of the required file observations are missing in this part.
pub(super) const REQUIRED_FILE_OBSERVATIONS_ARE_MISSING: &str = "required file observations are missing";
/// Text of the process observation requires an absolute utf in this part.
pub(super) const PROCESS_OBSERVATION_REQUIRES_AN_ABSOLUTE_UTF: &str = "process observation requires an absolute UTF8 request root";
/// Text of the process output slot is not a in this part.
pub(super) const PROCESS_OUTPUT_SLOT_IS_NOT_A: &str = "process output slot is not a declared safe input";
/// Text of the process input path is not utf8 in this part.
pub(super) const PROCESS_INPUT_PATH_IS_NOT_UTF8: &str = "process input path is not UTF8";
/// Text of the ryo case process v1 in this part.
pub(super) const RYO_CASE_PROCESS_V1: &[u8] = b"ryo-case-process-v1";
/// Text of the file in this part.
pub(super) const FILE: &str = "file";
/// Text of the directory in this part.
pub(super) const DIRECTORY: &str = "directory";
/// Text of the other in this part.
pub(super) const OTHER: &str = "other";
/// Text of the missing in this part.
pub(super) const MISSING: &str = "missing";
/// Text of the unknown in this part.
pub(super) const UNKNOWN: &str = "unknown";
/// Text of the path in this part.
pub(super) const PATH: &str = "path";
/// Text of the state in this part.
pub(super) const STATE: &str = "state";
/// Text of the project search roots in this part.
pub(super) const PROJECT_SEARCH_ROOTS: &str = "project_search_roots";
/// Text of the given project inventory in this part.
pub(super) const GIVEN_PROJECT_INVENTORY: &str = "given_project_inventory";
/// Text of the completeness in this part.
pub(super) const COMPLETENESS: &str = "completeness";
/// Text of the partial in this part.
pub(super) const PARTIAL: &str = "partial";
/// Text of the complete within declared scope in this part.
pub(super) const COMPLETE_WITHIN_DECLARED_SCOPE: &str = "complete-within-declared-scope";
/// Text of the entries in this part.
pub(super) const ENTRIES: &str = "entries";
/// Text of the src in this part.
pub(super) const SRC: &str = "src";
/// Text of the vendor in this part.
pub(super) const VENDOR: &str = "vendor";
/// Text of the src pkg in this part.
pub(super) const SRC_PKG: &str = "src/pkg";
/// Text of the vendor pkg py in this part.
pub(super) const VENDOR_PKG_PY: &str = "vendor/pkg.py";
/// Text of the digest in this part.
pub(super) const DIGEST: &str = "digest";
/// Text of the ff9b78535b775babaadae518c198876aa1c27ef21d0a in this part.
pub(super) const FF9B78535B775BABAADAE518C198876AA1C27EF21D0A: &str = "ff9b78535b775babaadae518c198876aa1c27ef21d0abb75e04588c25638df7d";
/// Text of the legacy in this part.
pub(super) const LEGACY: &str = "legacy";
/// Text of the original in this part.
pub(super) const ORIGINAL: &str = "original";
/// Text of the changed root authority in this part.
pub(super) const CHANGED_ROOT_AUTHORITY: &str = "changed root authority";
/// Text of the case rule configuration requires the profile in this part.
pub(super) const CASE_RULE_CONFIGURATION_REQUIRES_THE_PROFILE: &str = "case rule configuration requires the profile's explicit context declaration";
/// Text of the rule configuration tables must be distinct in this part.
pub(super) const RULE_CONFIGURATION_TABLES_MUST_BE_DISTINCT: &str = "rule configuration tables must be distinct nonblank declared names";
/// Text of the disabled rule lists contain each rule in this part.
pub(super) const DISABLED_RULE_LISTS_CONTAIN_EACH_RULE: &str = "disabled-rule lists contain each rule once";
/// Text of the rule context requires an explicit selected in this part.
pub(super) const RULE_CONTEXT_REQUIRES_AN_EXPLICIT_SELECTED: &str = "rule context requires an explicit selected-rule cohort";
/// Text of the disabled rules must belong to the in this part.
pub(super) const DISABLED_RULES_MUST_BELONG_TO_THE: &str = "disabled rules must belong to the configured selected cohort";
/// Text of the configured pair count overflows in this part.
pub(super) const CONFIGURED_PAIR_COUNT_OVERFLOWS: &str = "configured pair count overflows";
/// Text of the disabled pairs exceed the selected cohort in this part.
pub(super) const DISABLED_PAIRS_EXCEED_THE_SELECTED_COHORT: &str = "disabled pairs exceed the selected cohort";
/// Text of the explicit source inventory must name each in this part.
pub(super) const EXPLICIT_SOURCE_INVENTORY_MUST_NAME_EACH: &str = "explicit source inventory must name each selected source exactly once";
/// Text of the sealed disk context must select every in this part.
pub(super) const SEALED_DISK_CONTEXT_MUST_SELECT_EVERY: &str = "sealed disk context must select every staged readable source";
/// Text of the requested unavailable source lacks a context in this part.
pub(super) const REQUESTED_UNAVAILABLE_SOURCE_LACKS_A_CONTEXT: &str = "requested unavailable source lacks a context absence seal";
/// Text of the given source selection lacks a staged in this part.
pub(super) const GIVEN_SOURCE_SELECTION_LACKS_A_STAGED: &str = "Given source selection lacks a staged file";
/// Text of the unselected given source requires an explicit in this part.
pub(super) const UNSELECTED_GIVEN_SOURCE_REQUIRES_AN_EXPLICIT: &str = "unselected Given source requires an explicit project inventory";
/// Text of the unselected given source lacks a matching in this part.
pub(super) const UNSELECTED_GIVEN_SOURCE_LACKS_A_MATCHING: &str = "unselected Given source lacks a matching file observation";
/// Text of the file metadata names an undeclared source in this part.
pub(super) const FILE_METADATA_NAMES_AN_UNDECLARED_SOURCE: &str = "file metadata names an undeclared source";
/// Text of the reference discrepancies require full compari in this part.
pub(super) const REFERENCE_DISCREPANCIES_REQUIRE_FULL_COMPARI: &str = "reference discrepancies require full comparison";
/// Text of the selected rules must be nonempty and in this part.
pub(super) const SELECTED_RULES_MUST_BE_NONEMPTY_AND: &str = "selected-rules must be nonempty and contain each rule once";
/// Text of the artifact observations do not partition the in this part.
pub(super) const ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE: &str = "artifact observations do not partition the declared default inventory";
/// Text of the inventory must contain unique nonempty rule in this part.
pub(super) const INVENTORY_MUST_CONTAIN_UNIQUE_NONEMPTY_RULE: &str = "inventory must contain unique nonempty rule IDs";
/// Text of the not run in this part.
pub(super) const NOT_RUN: &str = "not run";
