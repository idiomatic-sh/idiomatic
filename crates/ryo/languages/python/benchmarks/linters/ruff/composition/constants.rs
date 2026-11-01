//! Runner command and artifact protocol.
/// Refuse profiling without an explicit trace path.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
/// Cargo-supplied harness argument ignored by the custom executable.
pub(crate) const CARGO_BENCH_ARGUMENT: &str = "--bench";
/// Child SDK execution command.
pub(crate) const RUN_RYO: &str = "run-ryo";
/// Separate trace-only child execution command.
pub(crate) const PROFILE_RYO: &str = "profile-ryo";
/// Required argument or refusal for missing worker request.
pub(crate) const MISSING_WORKER_REQUEST: &str = "missing worker request";
/// Complete profile fixture-verification command.
pub(crate) const VERIFY_CASES: &str = "verify-cases";
/// Check authored fixtures with the pinned reference only; never runs a candidate.
pub(crate) const VERIFY_REFERENCE_CASES: &str = "verify-reference-cases";
/// Required argument or refusal for expected ruff.
pub(crate) const EXPECTED_RUFF: &str = "expected Ruff executable";
/// Stable command, artifact or observation identity for profile directory.
pub(crate) const PROFILE_DIRECTORY: &str = "profile";
/// Stable command, artifact or observation identity for rules directory.
pub(crate) const RULES_DIRECTORY: &str = "rules";
/// Stable command, artifact or observation identity for cases directory.
pub(crate) const CASES_DIRECTORY: &str = "cases";
/// Accounted full-input measurement command.
pub(crate) const MEASURE: &str = "measure";
/// Interim subset measurement command.
pub(crate) const MEASURE_SCOPED: &str = "measure-scoped";
/// Interim subset command with explicitly audited invalid-source exclusions.
pub(crate) const MEASURE_VALID: &str = "measure-scoped-valid";
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
pub(crate) const USAGE: &str = "usage: ruff benchmark: audit | verify-tool <executable> | verify-cases <ruff> [profile] | verify-reference-cases <ruff> [profile] | measure <ruff> <fresh-cache> | run-ryo <json>";
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
