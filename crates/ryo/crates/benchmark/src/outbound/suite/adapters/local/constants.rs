//! Artifact filenames, executable protocol and refusal vocabulary.
/// Shared filename for tool profiles and language workload manifests.
pub(super) const MANIFEST: &str = "benchmark.toml";
/// Directory containing authored rule definitions.
pub(super) const RULES: &str = "rules";
/// Executable RyoQL source filename within one rule directory.
pub(super) const QUERY: &str = "query.ryoql";
/// Independent fixture filename within one rule directory.
pub(super) const CASES: &str = "cases.toml";
/// Reserved retained-state directory name.
pub(super) const CACHE: &str = "cache";
/// Reference tool argument used to verify its reported version.
pub(super) const VERSION_ARGUMENT: &str = "--version";
/// Refusal for an empty or non-unique pinned default inventory.
pub(super) const INVALID_DEFAULTS: &str = "empty or duplicated default rule inventory";
/// Audit refusal when a rule directory name is not UTF-8.
pub(super) const INVALID_RULE_ID: &str = "non-UTF8 rule id";
/// Case preparation refusal when a rule directory name is not UTF-8.
pub(super) const INVALID_RULE_ID_TEXT: &str = "non-UTF8 rule";
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
/// Refusal when authored artifacts provide no executable independent examples.
pub(super) const NO_EXECUTABLE_CASES: &str = "no executable rule cases";
