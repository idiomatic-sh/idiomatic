//! Runner command and artifact protocol.
/// Refuse profiling without an explicit trace path.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
/// Cargo-supplied harness argument ignored by the custom executable.
pub(crate) const CARGO_BENCH_ARGUMENT: &str = "--bench";
/// Complete profile fixture-verification command.
pub(crate) const VERIFY_CASES: &str = "verify-cases";
/// Check authored fixtures with the pinned reference only; never runs a candidate.
pub(crate) const VERIFY_REFERENCE_CASES: &str = "verify-reference-cases";
/// Required argument or refusal for expected ruff.
pub(crate) const EXPECTED_RUFF: &str = "expected Ruff executable";
/// Stable command, artifact or observation identity for profile directory.
pub(crate) const PROFILE_DIRECTORY: &str = "profile";
/// Stable command, artifact or observation identity for cases directory.
pub(crate) const CASES_DIRECTORY: &str = "cases";
/// Accounted full-input measurement command.
pub(crate) const MEASURE: &str = "measure";
/// Interim subset measurement command.
pub(crate) const MEASURE_SCOPED: &str = "diagnostic-measure";
/// Interim subset command with explicitly audited invalid-source exclusions.
pub(crate) const MEASURE_VALID: &str = "diagnostic-measure-valid";
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
pub(crate) const PYTHON: &str = "python";
/// Required argument or refusal for project count error.
pub(crate) const PROJECT_COUNT_ERROR: &str = "Python suite requires 2–3 projects";
/// Stable command, artifact or observation identity for projects directory.
pub(crate) const PROJECTS_DIRECTORY: &str = "projects";
/// Native Python source suffix admitted by the matched input profile.
pub(crate) const PYTHON_EXTENSION: &str = ".py";
/// Python stub suffix admitted by the matched input profile.
pub(crate) const STUB_EXTENSION: &str = ".pyi";
/// Required argument or refusal for untracked source.
pub(crate) const UNTRACKED_SOURCE: &str = "Ruff discovered untracked source";
/// Stable command, artifact or observation identity for preflight directory.
pub(crate) const PREFLIGHT_DIRECTORY: &str = "preflight";
/// Stable command, artifact or observation identity for invalid syntax.
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
/// Stable command, artifact or observation identity for valid source profile.
pub(crate) const VALID_SOURCE_PROFILE: &str = "pinned-reference-valid-python";
/// Stable command, artifact or observation identity for full source profile.
pub(crate) const FULL_SOURCE_PROFILE: &str = "all-python-sources";
/// Stable command, artifact or observation identity for trial prefix.
pub(crate) const TRIAL_PREFIX: &str = "trial-";
/// Stable command, artifact or observation identity for retained directory.
pub(crate) const RETAINED_DIRECTORY: &str = "retained";
/// Stable command, artifact or observation identity for clean edited directory.
pub(crate) const CLEAN_EDITED_DIRECTORY: &str = "clean-edited";
/// Stable command, artifact or observation identity for empty cache condition.
pub(crate) const EMPTY_CACHE_CONDITION: &str = "empty-tool-cache";
/// Stable command, artifact or observation identity for retained cache condition.
pub(crate) const RETAINED_CACHE_CONDITION: &str = "retained-tool-cache";
/// Stable command, artifact or observation identity for edited cache condition.
pub(crate) const EDITED_CACHE_CONDITION: &str = "edited-tool-cache";
/// Stable command, artifact or observation identity for reference name.
pub(crate) const REFERENCE_NAME: &str = "ruff";
/// Stable command, artifact or observation identity for candidate name.
pub(crate) const CANDIDATE_NAME: &str = "ryo";
/// Stable command, artifact or observation identity for audit.
pub(crate) const AUDIT: &str = "audit";
/// Stable command, artifact or observation identity for verify tool.
pub(crate) const VERIFY_TOOL: &str = "verify-tool";
/// Required argument or refusal for expected executable.
pub(crate) const EXPECTED_EXECUTABLE: &str = "expected executable path";
/// CLI usage shown when command dispatch fails.
pub(crate) const USAGE: &str = "usage: ruff benchmark: audit | verify-tool <executable> | verify-cases <ruff> [profile] | verify-reference-cases <ruff> [profile] | measure <ruff> <fresh-cache> | run-ryo (<json>|--request-file <path>) <python-target=3.MINOR> | diagnostic-ryo (<json>|--request-file <path>) <python-target=3.MINOR> <events.jsonl> (MINOR:7..15)";
/// Stable command, artifact or observation identity for ruff directory.
pub(crate) const RUFF_DIRECTORY: &str = "benchmarks/linters/ruff";
/// Accounted measurement command with explicitly audited invalid-source exclusions.
pub(crate) const MEASURE_ACCOUNTED_VALID: &str = "measure-valid";
/// Increment for one diagnostic occurrence.
pub(crate) const ONE_FINDING: usize = 1;
/// Cargo-provided language crate root.
pub(crate) const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
/// Stable observation label for reference-first execution.
pub(crate) const REFERENCE_FIRST: &str = "ReferenceFirst";
/// Stable observation label for candidate-first execution.
pub(crate) const CANDIDATE_FIRST: &str = "CandidateFirst";
/// Accounted-profile coverage label.
pub(crate) const ACCOUNTED: &str = "Accounted";
/// Explicit interim-profile coverage label.
pub(crate) const INTERIM: &str = "Interim";
/// Separator in human-readable excluded-path observations.
pub(crate) const PATH_SEPARATOR: &str = ", ";
/// Separator between explicit per-rule finding counts.
pub(crate) const FINDING_SEPARATOR: &str = ", ";
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
/// Text `python-target=`.
pub(crate) const PYTHON_TARGET: &str = "python-target=";
/// Text `unexpected worker configuration`.
pub(crate) const UNEXPECTED_WORKER_CONFIGURATION: &str = "unexpected worker configuration";
/// Text `unexpected trailing worker argument`.
pub(crate) const UNEXPECTED_TRAILING_WORKER_ARGUMEN: &str = "unexpected trailing worker argument";
/// Text `case receipt destination must be none...`.
pub(crate) const CASE_RECEIPT_DESTINATION_MUST_BE_N: &str =
    "case receipt destination must be nonempty and supplied once";
/// Text `--shard=`.
pub(crate) const SHARD: &str = "--shard=";
/// Text `duplicate case shard argument`.
pub(crate) const DUPLICATE_CASE_SHARD_ARGUMENT: &str = "duplicate case shard argument";
/// Text `--case-selection=`.
pub(crate) const CASE_SELECTION: &str = "--case-selection=";
/// Text `duplicate case selection argument`.
pub(crate) const DUPLICATE_CASE_SELECTION_ARGUMENT: &str = "duplicate case selection argument";
/// Text `case selection must be nonempty and u...`.
pub(crate) const CASE_SELECTION_MUST_BE_NONEMPTY_AN: &str =
    "case selection must be nonempty and unique";
/// Text `unexpected verify-cases argument`.
pub(crate) const UNEXPECTED_VERIFY_CASES_ARGUMENT: &str = "unexpected verify-cases argument";
/// Text `expected --shard=INDEX/COUNT`.
pub(crate) const EXPECTED_SHARD_INDEX_COUNT: &str = "expected --shard=INDEX/COUNT";
/// Text `invalid case shard index`.
pub(crate) const INVALID_CASE_SHARD_INDEX: &str = "invalid case shard index";
/// Text `invalid case shard count`.
pub(crate) const INVALID_CASE_SHARD_COUNT: &str = "invalid case shard count";
/// Text `pinned reference does not evaluate PO...`.
pub(crate) const PINNED_REFERENCE_DOES_NOT_EVALUATE: &str =
    "pinned reference does not evaluate POSIX executable checks on this host";
/// Text `linux`.
pub(crate) const LINUX: &str = "linux";
/// Text `/proc/sys/kernel/osrelease`.
pub(crate) const PROC_SYS_KERNEL_OSRELEASE: &str = "/proc/sys/kernel/osrelease";
/// Text `/proc/version`.
pub(crate) const PROC_VERSION: &str = "/proc/version";
/// Text `microsoft`.
pub(crate) const MICROSOFT: &str = "microsoft";
/// Text `/.dockerenv`.
pub(crate) const DOCKERENV: &str = "/.dockerenv";
/// Text `/run/.containerenv`.
pub(crate) const RUN_CONTAINERENV: &str = "/run/.containerenv";
/// Text `/proc/self/cgroup`.
pub(crate) const PROC_SELF_CGROUP: &str = "/proc/self/cgroup";
/// Text `docker`.
pub(crate) const DOCKER: &str = "docker";
/// Text `/container`.
pub(crate) const CONTAINER: &str = "/container";
/// Text `/lxc`.
pub(crate) const LXC: &str = "/lxc";
/// Text `kubepods`.
pub(crate) const KUBEPODS: &str = "kubepods";
