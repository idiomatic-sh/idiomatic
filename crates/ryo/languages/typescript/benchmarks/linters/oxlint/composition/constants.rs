//! Runner command and artifact protocol.
/// Refuse profiling without an explicit trace path.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
/// Cargo-supplied harness argument ignored by the custom executable.
pub(crate) const CARGO_BENCH_ARGUMENT: &str = "--bench";
/// Complete profile fixture-verification command.
pub(crate) const VERIFY_CASES: &str = "verify-cases";
/// Check authored fixtures with the pinned reference only; never runs a candidate.
pub(crate) const VERIFY_REFERENCE_CASES: &str = "verify-reference-cases";
/// Required argument or refusal for the oxlint executable.
pub(crate) const EXPECTED_OXLINT: &str = "expected oxlint executable";
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
pub(crate) const TYPESCRIPT: &str = "typescript";
/// Stable command, artifact or observation identity for projects directory.
pub(crate) const PROJECTS_DIRECTORY: &str = "projects";
/// Stable command, artifact or observation identity for valid source profile.
pub(crate) const VALID_SOURCE_PROFILE: &str = "pinned-reference-valid-typescript";
/// Stable command, artifact or observation identity for full source profile.
pub(crate) const FULL_SOURCE_PROFILE: &str = "all-typescript-sources";
/// Stable command, artifact or observation identity for reference name.
pub(crate) const REFERENCE_NAME: &str = "oxlint";
/// Stable command, artifact or observation identity for audit.
pub(crate) const AUDIT: &str = "audit";
/// Stable command, artifact or observation identity for verify tool.
pub(crate) const VERIFY_TOOL: &str = "verify-tool";
/// The command comparing the configured rules over a staged request, rule by rule.
pub(crate) const PARITY: &str = "parity";
/// Refusal of a parity command line that is not its two paths.
pub(crate) const PARITY_ARGUMENTS: &str = "expected parity <oxlint> <output>";
/// The state directory of one project's parity, below its output.
pub(crate) const PARITY_STATE: &str = "state";
/// One project's per-rule parity document, below its output.
pub(crate) const PER_RULE: &str = "per-rule.json";
/// Where a project's accepted differences are, relative to this bench.
pub(crate) const EVIDENCE_DIRECTORY: &str = "profiles/evidence";
/// The suffix of a project's accepted-differences file, after the project's name.
pub(crate) const ACCEPTED_SUFFIX: &str = "-accepted-differences.json";
/// The refusal when a stream's lock was poisoned by a panic.
pub(crate) const POISONED_STREAM: &str = "parity stream lock poisoned";
/// Required argument or refusal for expected executable.
pub(crate) const EXPECTED_EXECUTABLE: &str = "expected executable path";
/// CLI usage shown when command dispatch fails.
pub(crate) const USAGE: &str = "usage: oxlint benchmark: audit | verify-tool <executable> | verify-cases <oxlint> [profile] | verify-reference-cases <oxlint> [profile] | measure <oxlint> <fresh-cache> <collector> | parity <oxlint> <output> | run-ryo (<json>|--request-file <path>) | profile-ryo (<json>|--request-file <path>) <trace.json> | diagnostic-ryo (<json>|--request-file <path>) <events.jsonl>";
/// This benchmark package's root, where its rules and fragments are.
pub(crate) const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
/// Text `unsupported worker command`.
pub(crate) const UNSUPPORTED_WORKER_COMMAND: &str = "unsupported worker command";
/// Text `--case-inventory=`.
pub(crate) const CASE_INVENTORY: &str = "--case-inventory=";
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
/// The suite boundary's own default location for a scratch directory.
pub(crate) const DEFAULT_SCRATCH_ROOT: &str = "";
/// The language as the corpus observations name it.
pub(crate) const LANGUAGE_DISPLAY: &str = "TypeScript";
/// The pinned reference tool as the corpus observations name it.
pub(crate) const REFERENCE_DISPLAY: &str = "oxlint";
/// TypeScript source suffix.
const TS_EXTENSION: &str = ".ts";
/// TypeScript ES module source suffix.
const MTS_EXTENSION: &str = ".mts";
/// TypeScript CommonJS module source suffix.
const CTS_EXTENSION: &str = ".cts";
/// TypeScript JSX source suffix.
const TSX_EXTENSION: &str = ".tsx";
/// The suffixes of the sources the matched input profile admits: TypeScript only, the one language the worker links.
pub(crate) const SOURCE_SUFFIXES: [&str; 4] =
    [TS_EXTENSION, MTS_EXTENSION, CTS_EXTENSION, TSX_EXTENSION];
/// Stable command, artifact or observation identity for rules directory.
pub(crate) const RULES_DIRECTORY: &str = "rules";
