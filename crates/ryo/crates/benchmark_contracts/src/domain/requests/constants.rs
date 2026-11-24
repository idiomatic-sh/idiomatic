//! Request coordinate refusal vocabulary.
pub(super) const INVALID_SHARD: &str = "invalid case shard coordinates";
/// The applicability policy under which a rule runs on every file.
pub const ALWAYS: &str = "always";
/// The applicability policy under which a rule is skipped on a file the parser rejects.
pub const ACCEPTED_SYNTAX: &str = "accepted-syntax";
/// The filename of a rule's query inside its rule directory.
pub const QUERY_FILE: &str = "query.ryoql";
/// The module source a logical module name is probed with before the real source is read.
pub const MODULE_NAME_PROBE: &str = "module source";
/// The filename of a profile's manifest inside its profile directory.
pub const PROFILE_MANIFEST: &str = "benchmark.toml";
/// The filename of a rule's cases inside its rule directory.
pub const CASES_FILE: &str = "cases.toml";
/// The identity key of a sealed worker's digest.
pub const WORKER_SHA256: &str = "worker_sha256";
/// The worker argument naming the request file the parent wrote for it.
pub const REQUEST_FILE: &str = "--request-file";
/// The timed worker command of a linter's benchmark executable: no tracing.
pub const RUN_RYO: &str = "run-ryo";
/// The diagnostic worker command: flushed JSONL events.
pub const DIAGNOSTIC_RYO: &str = "diagnostic-ryo";
/// The accounted full-input measure command.
pub const MEASURE: &str = "measure";
/// The accounted measure command with explicitly audited invalid-source exclusions.
pub const MEASURE_ACCOUNTED_VALID: &str = "measure-valid";
/// The worker command that checks a bench's authored cases with the pinned reference only; it never runs a candidate.
pub const VERIFY_REFERENCE_CASES: &str = "verify-reference-cases";
/// The interim subset measure command.
pub const MEASURE_SCOPED: &str = "diagnostic-measure";
/// The interim subset measure command with explicitly audited invalid-source exclusions.
pub const MEASURE_VALID: &str = "diagnostic-measure-valid";
/// The worker command that verifies the reference tool's pinned bytes and its version line, once for a bench run.
pub const VERIFY_TOOL: &str = "verify-tool";
/// The option naming the registration a measure is certified against, followed by its path.
pub const CASE_INVENTORY_ARGUMENT: &str = "--case-inventory=";
/// The option naming the directory the bench's verification left its receipts and each rule's times in, followed by its
/// path.
pub const VERIFICATION_ARGUMENT: &str = "--verification=";
/// The option naming the file a bench's worker writes its one document to, followed by its path: the worker's standard
/// output carries anything else, so it is never the document.
pub const DOCUMENT_ARGUMENT: &str = "--document=";
