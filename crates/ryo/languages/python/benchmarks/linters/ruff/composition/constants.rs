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
/// Stable command, artifact or observation identity for invalid syntax.
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
/// Stable command, artifact or observation identity for valid source profile.
pub(crate) const VALID_SOURCE_PROFILE: &str = "pinned-reference-valid-python";
/// Stable command, artifact or observation identity for full source profile.
pub(crate) const FULL_SOURCE_PROFILE: &str = "all-python-sources";
/// Stable command, artifact or observation identity for reference name.
pub(crate) const REFERENCE_NAME: &str = "ruff";
/// Stable command, artifact or observation identity for audit.
pub(crate) const AUDIT: &str = "audit";
/// Stable command, artifact or observation identity for verify tool.
pub(crate) const VERIFY_TOOL: &str = "verify-tool";
/// The command comparing the configured rules over a staged request, rule by rule.
pub(crate) const PARITY: &str = "parity";
/// Refusal of a parity command line that is not its five paths.
pub(crate) const PARITY_ARGUMENTS: &str =
    "expected parity <ruff> <ryo-worker> <staged-request> <accepted-differences> <per-rule-output>";
/// Name prefix of the scratch directory both parity contestants keep their state in.
pub(crate) const PARITY_SCRATCH: &str = "parity-";
/// Required argument or refusal for expected executable.
pub(crate) const EXPECTED_EXECUTABLE: &str = "expected executable path";
/// CLI usage shown when command dispatch fails.
pub(crate) const USAGE: &str = "usage: ruff benchmark: audit | verify-tool <executable> | verify-cases <ruff> [profile] | verify-reference-cases <ruff> [profile] | measure <ruff> <fresh-cache> | parity <ruff> <ryo-worker> <staged-request> <accepted-differences> <per-rule-output> | run-ryo (<json>|--request-file <path>) <python-target=3.MINOR> | diagnostic-ryo (<json>|--request-file <path>) <python-target=3.MINOR> <events.jsonl> (MINOR:7..15)";
/// Accounted measurement command with explicitly audited invalid-source exclusions.
pub(crate) const MEASURE_ACCOUNTED_VALID: &str = "measure-valid";
/// This benchmark package's root, where its rules, fragments and profiles are.
pub(crate) const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
/// Accounted-profile coverage label.
pub(crate) const ACCOUNTED: &str = "Accounted";
/// Explicit interim-profile coverage label.
pub(crate) const INTERIM: &str = "Interim";
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
/// Text `pinned reference does not evaluate PO...`.
pub(crate) const PINNED_REFERENCE_DOES_NOT_EVALUATE: &str =
    "pinned reference does not evaluate POSIX executable checks on this host";

/// Name prefix of the scratch directory one fixture verification stages into.
pub(crate) const VERIFY_CASES_SCRATCH: &str = "verify-cases-";
/// Name prefix of the scratch directory one reference-only verification stages into.
pub(crate) const VERIFY_REFERENCE_SCRATCH: &str = "verify-reference-";
/// Name prefix of the scratch directory one certification stages its cases into.
pub(crate) const CASE_SCRATCH: &str = "case-scratch-";
/// The suite boundary's own default location for a scratch directory.
pub(crate) const DEFAULT_SCRATCH_ROOT: &str = "";
/// The language as the corpus observations name it.
pub(crate) const LANGUAGE_DISPLAY: &str = "Python";
/// The pinned reference tool as the corpus observations name it.
pub(crate) const REFERENCE_DISPLAY: &str = "Ruff";
/// Native Python source suffix admitted by the matched input profile.
const PYTHON_EXTENSION: &str = ".py";
/// Python stub suffix admitted by the matched input profile.
const STUB_EXTENSION: &str = ".pyi";
/// The suffixes of the sources the matched input profile admits.
pub(crate) const SOURCE_SUFFIXES: [&str; 2] = [PYTHON_EXTENSION, STUB_EXTENSION];
/// Stable command, artifact or observation identity for rules directory.
pub(crate) const RULES_DIRECTORY: &str = "rules";
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
