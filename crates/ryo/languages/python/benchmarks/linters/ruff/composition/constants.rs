//! Runner command and artifact protocol.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
pub(crate) const CARGO_BENCH_ARGUMENT: &str = "--bench";
pub(crate) const RUN_RYO: &str = "run-ryo";
pub(crate) const PROFILE_RYO: &str = "profile-ryo";
pub(crate) const MISSING_WORKER_REQUEST: &str = "missing worker request";
pub(crate) const VERIFY_CASES: &str = "verify-cases";
pub(crate) const EXPECTED_RUFF: &str = "expected Ruff executable";
pub(crate) const PROFILE_DIRECTORY: &str = "profile";
pub(crate) const RULES_DIRECTORY: &str = "rules";
pub(crate) const CASES_DIRECTORY: &str = "cases";
pub(crate) const MEASURE: &str = "measure";
pub(crate) const MEASURE_SCOPED: &str = "measure-scoped";
pub(crate) const MEASURE_VALID: &str = "measure-scoped-valid";
pub(crate) const RELEASE_REQUIRED: &str =
    "measurement requires a release-profile benchmark executable";
pub(crate) const EXPECTED_CACHE: &str = "expected owned cache directory";
pub(crate) const CERTIFICATION_PREFIX: &str = "certification-";
pub(crate) const WORKLOAD_DIRECTORY: &str = "workload";
pub(crate) const CANDIDATE_EXECUTABLE: &str = "candidate";
pub(crate) const COLLECTOR_EXECUTABLE: &str = "collector";
pub(crate) const COLLECTOR_IDENTITY: &str = "collector.sha256";
pub(crate) const EXPECTED_COLLECTOR: &str = "expected resource collector executable";
pub(crate) const LANGUAGE_DIRECTORY: &str = "../..";
pub(crate) const PYTHON: &str = "python";
pub(crate) const PROJECT_COUNT_ERROR: &str = "Python suite requires 2–3 projects";
pub(crate) const PROJECTS_DIRECTORY: &str = "projects";
pub(crate) const PYTHON_EXTENSION: &str = ".py";
pub(crate) const STUB_EXTENSION: &str = ".pyi";
pub(crate) const UNTRACKED_SOURCE: &str = "Ruff discovered untracked source";
pub(crate) const PREFLIGHT_DIRECTORY: &str = "preflight";
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
pub(crate) const VALID_SOURCE_PROFILE: &str = "pinned-reference-valid-python";
pub(crate) const FULL_SOURCE_PROFILE: &str = "all-python-sources";
pub(crate) const TRIAL_PREFIX: &str = "trial-";
pub(crate) const RETAINED_DIRECTORY: &str = "retained";
pub(crate) const CLEAN_EDITED_DIRECTORY: &str = "clean-edited";
pub(crate) const EMPTY_CACHE_CONDITION: &str = "empty-tool-cache";
pub(crate) const RETAINED_CACHE_CONDITION: &str = "retained-tool-cache";
pub(crate) const EDITED_CACHE_CONDITION: &str = "edited-tool-cache";
pub(crate) const REFERENCE_NAME: &str = "ruff";
pub(crate) const CANDIDATE_NAME: &str = "ryo";
pub(crate) const AUDIT: &str = "audit";
pub(crate) const VERIFY_TOOL: &str = "verify-tool";
pub(crate) const EXPECTED_EXECUTABLE: &str = "expected executable path";
pub(crate) const USAGE: &str = "usage: ruff benchmark: audit | verify-tool <executable> | measure <ruff> <fresh-cache> | run-ryo <json>";
pub(crate) const RUFF_DIRECTORY: &str = "benchmarks/linters/ruff";

pub(crate) const MEASURE_ACCOUNTED_VALID: &str = "measure-valid";

pub(crate) const ONE_FINDING: usize = 1;

pub(crate) const MANIFEST_DIRECTORY: &str = env!("CARGO_MANIFEST_DIR");
