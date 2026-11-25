//! Runner command and artifact protocol.
/// Refuse profiling without an explicit trace path.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
/// Required argument or refusal for the clippy executable.
pub(crate) const EXPECTED_CLIPPY: &str = "expected clippy executable";
/// Stable command, artifact or observation identity for profile directory.
pub(crate) const PROFILE_DIRECTORY: &str = "profile";
/// Stable command, artifact or observation identity for cases directory.
pub(crate) const CASES_DIRECTORY: &str = "cases";
/// Required argument or refusal for release required.
pub(crate) const RELEASE_REQUIRED: &str =
    "measurement requires a release-profile benchmark executable";
/// Required argument or refusal for expected cache.
pub(crate) const EXPECTED_CACHE: &str = "expected owned cache directory";
/// Stable command, artifact or observation identity for certification prefix.
pub(crate) const CERTIFICATION_PREFIX: &str = "certification-";
/// Stable command, artifact or observation identity for workload directory.
pub(crate) const WORKLOAD_DIRECTORY: &str = "workload";
/// Stable command, artifact or observation identity for candidate executable.
pub(crate) const CANDIDATE_EXECUTABLE: &str = "candidate";
/// Stable command, artifact or observation identity for collector executable.
pub(crate) const COLLECTOR_EXECUTABLE: &str = "collector";
/// Stable command, artifact or observation identity for collector identity.
pub(crate) const COLLECTOR_IDENTITY: &str = "collector.sha256";
/// Required argument or refusal for expected collector.
pub(crate) const EXPECTED_COLLECTOR: &str = "expected resource collector executable";
/// Shared language workload relative to the linter profile.
pub(crate) const LANGUAGE_DIRECTORY: &str = "../..";
/// Language identity required by this composition.
pub(crate) const RUST: &str = "rust";
/// Stable command, artifact or observation identity for projects directory.
pub(crate) const PROJECTS_DIRECTORY: &str = "projects";
/// Stable command, artifact or observation identity for valid source profile.
pub(crate) const VALID_SOURCE_PROFILE: &str = "pinned-reference-valid-rust";
/// Stable command, artifact or observation identity for audit.
pub(crate) const AUDIT: &str = "audit";
/// Required argument or refusal for expected executable.
pub(crate) const EXPECTED_EXECUTABLE: &str = "expected executable path";
/// CLI usage shown when command dispatch fails.
pub(crate) const USAGE: &str = "usage: clippy benchmark: audit | verify-tool <executable> | verify-cases <clippy> [profile] | verify-reference-cases <clippy> [profile] | measure <clippy> <fresh-cache> <collector> | run-ryo (<json>|--request-file <path>) | diagnostic-ryo (<json>|--request-file <path>) <events.jsonl>";
/// This benchmark package's root, where its rules and fragments are.
pub(crate) const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
/// Text `unsupported worker command`.
pub(crate) const UNSUPPORTED_WORKER_COMMAND: &str = "unsupported worker command";
/// Text `canonical benchmark requires explicit...`.
pub(crate) const CANONICAL_BENCHMARK_REQUIRES_EXPLI: &str =
    "canonical benchmark requires explicit registered --case-inventory=PATH";
/// Text `registered-case-results-`.
pub(crate) const REGISTERED_CASE_RESULTS: &str = "registered-case-results-";
/// Text `unexpected measurement argument`.
pub(crate) const UNEXPECTED_MEASUREMENT_ARGUMENT: &str = "unexpected measurement argument";
/// Text `unexpected trailing worker argument`.
pub(crate) const UNEXPECTED_TRAILING_WORKER_ARGUMENT: &str = "unexpected trailing worker argument";
/// Name prefix of the scratch directory one fixture verification stages into.
pub(crate) const VERIFY_CASES_SCRATCH: &str = "verify-cases-";
/// Name prefix of the scratch directory one reference-only verification stages into.
pub(crate) const VERIFY_REFERENCE_SCRATCH: &str = "verify-reference-";
/// Name prefix of the scratch directory one certification stages its cases into.
pub(crate) const CASE_SCRATCH: &str = "case-scratch-";
/// The directory, under a command's scratch, of the target directory its clippy runs share.
pub(crate) const TARGETS_DIRECTORY: &str = "targets";
/// The suite boundary's own default location for a scratch directory.
pub(crate) const DEFAULT_SCRATCH_ROOT: &str = "";
/// The language as the corpus observations name it.
pub(crate) const LANGUAGE_DISPLAY: &str = "Rust";
/// The pinned reference tool as the corpus observations name it.
pub(crate) const REFERENCE_DISPLAY: &str = "clippy";
/// Rust source suffix.
const RS_EXTENSION: &str = ".rs";
/// The suffixes of the sources the matched input profile admits: Rust only, the one language the worker links.
pub(crate) const SOURCE_SUFFIXES: [&str; 1] = [RS_EXTENSION];
/// Stable command, artifact or observation identity for rules directory.
pub(crate) const RULES_DIRECTORY: &str = "rules";
/// Refusal of a measure without the file its document is written to.
pub(crate) const EXPECTED_DOCUMENT: &str = "expected --document=<file>";
/// Refusal of a measure without the verification's directory.
pub(crate) const EXPECTED_VERIFICATION: &str = "expected --verification=<inventory directory>";
