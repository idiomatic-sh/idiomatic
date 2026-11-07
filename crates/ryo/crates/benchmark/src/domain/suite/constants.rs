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
/// Text of the explicit source inventory must name each in this part.
pub(super) const EXPLICIT_SOURCE_INVENTORY_MUST_NAME_EACH: &str =
    "explicit source inventory must name each selected source exactly once";
/// Text of the sealed disk context must select every in this part.
pub(super) const SEALED_DISK_CONTEXT_MUST_SELECT_EVERY: &str =
    "sealed disk context must select every staged readable source";
/// Text of the requested unavailable source lacks a context in this part.
pub(super) const REQUESTED_UNAVAILABLE_SOURCE_LACKS_A_CONTEXT: &str =
    "requested unavailable source lacks a context absence seal";
/// Text of the given source selection lacks a staged in this part.
pub(super) const GIVEN_SOURCE_SELECTION_LACKS_A_STAGED: &str =
    "Given source selection lacks a staged file";
/// Text of the unselected given source requires an explicit in this part.
pub(super) const UNSELECTED_GIVEN_SOURCE_REQUIRES_AN_EXPLICIT: &str =
    "unselected Given source requires an explicit project inventory";
/// Text of the unselected given source lacks a matching in this part.
pub(super) const UNSELECTED_GIVEN_SOURCE_LACKS_A_MATCHING: &str =
    "unselected Given source lacks a matching file observation";
/// Text of the file metadata names an undeclared source in this part.
pub(super) const FILE_METADATA_NAMES_AN_UNDECLARED_SOURCE: &str =
    "file metadata names an undeclared source";
/// Text of the reference discrepancies require full compari in this part.
pub(super) const REFERENCE_DISCREPANCIES_REQUIRE_FULL_COMPARI: &str =
    "reference discrepancies require full comparison";
/// Text of the selected rules must be nonempty and in this part.
pub(super) const SELECTED_RULES_MUST_BE_NONEMPTY_AND: &str =
    "selected-rules must be nonempty and contain each rule once";
/// Text of the artifact observations do not partition the in this part.
pub(super) const ARTIFACT_OBSERVATIONS_DO_NOT_PARTITION_THE: &str =
    "artifact observations do not partition the declared default inventory";
/// Text of the inventory must contain unique nonempty rule in this part.
pub(super) const INVENTORY_MUST_CONTAIN_UNIQUE_NONEMPTY_RULE: &str =
    "inventory must contain unique nonempty rule IDs";
/// Text of the not run in this part.
pub(super) const NOT_RUN: &str = "not run";
/// The pinned linter profile file inside a profile directory.
pub(crate) const BENCHMARK_TOML: &str = "benchmark.toml";
/// The authored query file inside a rule directory.
pub(crate) const QUERY_RYOQL: &str = "query.ryoql";
/// A registered rule whose command is absent cannot be verified.
pub(crate) const MISSING_VERIFICATION_COMMAND: &str = "missing verification command";
/// The schema every written observation row follows.
pub(crate) const BENCHMARK_CASE_V2: &str = "benchmark-case/v2";
/// Refuse certification without executable cases.
pub(crate) const NO_CASES: &str = "no executable rule cases";
/// Separate complete fixture failure records.
pub(crate) const LINE_SEPARATOR: &str = "\n";
/// A process-failure reference can only be verified by a direct no-deadline observer.
pub(crate) const PROCESS_FAILURE_VERIFICATION_REQUIRES_A_DIRE: &str =
    "process-failure verification requires a direct no-deadline observer";
/// A reference that decodes a report did not fail as its case declares.
pub(crate) const REFERENCE_RETURNED_A_DECODED_REPORT_INSTEAD: &str =
    "reference returned a decoded report instead of the declared process failure";
/// The authored rule directory inside a linter profile.
pub(crate) const RULES: &str = "rules";
