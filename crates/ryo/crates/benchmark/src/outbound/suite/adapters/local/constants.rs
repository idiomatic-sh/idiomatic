//! Artifact filenames, executable protocol and refusal vocabulary.
pub(super) const MANIFEST: &str = "benchmark.toml";
pub(super) const RULES: &str = "rules";
pub(super) const QUERY: &str = "query.ryoql";
pub(super) const CASES: &str = "cases.toml";
pub(super) const CACHE: &str = "cache";
pub(super) const VERSION_ARGUMENT: &str = "--version";
pub(super) const INVALID_DEFAULTS: &str = "empty or duplicated default rule inventory";
pub(super) const INVALID_RULE_ID: &str = "non-UTF8 rule id";
pub(super) const INVALID_RULE_ID_TEXT: &str = "non-UTF8 rule";
pub(super) const INVALID_EVIDENCE_PATH: &str = "non-UTF8 evidence path";
pub(super) const INVALID_EVIDENCE_KIND: &str =
    "evidence paths must be regular files or directories";
pub(super) const INVALID_DIGEST_KIND: &str = "executable digest requires a regular file";
pub(super) const TOOL_HASH_MISMATCH: &str = "tool executable hash differs from pin";
pub(super) const TOOL_VERSION_MISMATCH: &str = "tool version differs from pin";
pub(super) const PATCH_ESCAPE: &str = "patch escapes the shared suite directory";
pub(super) const MISSING_CASE_PARENT: &str = "missing case parent";
pub(super) const NO_EXECUTABLE_CASES: &str = "no executable rule cases";
