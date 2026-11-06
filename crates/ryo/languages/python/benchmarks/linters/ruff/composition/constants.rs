//! Runner command and artifact protocol.
/// Refuse profiling without an explicit trace path.
pub(crate) const MISSING_TRACE: &str = "expected explicit trace output path";
/// Cargo-supplied harness argument ignored by the custom executable.
pub(crate) const CARGO_BENCH_ARGUMENT: &str = "--bench";
/// Child SDK execution command.
pub(crate) const RUN_RYO: &str = "run-ryo";
/// Separate trace-only child execution command.
pub(crate) const PROFILE_RYO: &str = "profile-ryo";
/// Diagnostic-only worker that flushes rule/file events as JSONL.
pub(crate) const DIAGNOSTIC_RYO: &str = "diagnostic-ryo";
/// Required argument or refusal for missing worker request.
pub(crate) const MISSING_WORKER_REQUEST: &str = "missing worker request";
/// Child argument that carries a request from an owned JSON file.
pub(crate) const REQUEST_FILE: &str = "--request-file";
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
/// Text `worker requires an explicit supported...`.
pub(crate) const WORKER_REQUIRES_AN_EXPLICIT_SUPPOR: &str =
    "worker requires an explicit supported parser configuration";
/// Text `parser_configuration`.
pub(crate) const PARSER_CONFIGURATION: &str = "parser_configuration";
/// Text `worker parser configuration differs f...`.
pub(crate) const WORKER_PARSER_CONFIGURATION_DIFFER: &str =
    "worker parser configuration differs from its profile identity";
/// Text `rule_applicability`.
pub(crate) const RULE_APPLICABILITY: &str = "rule_applicability";
/// Text `worker requires explicit selected rul...`.
pub(crate) const WORKER_REQUIRES_EXPLICIT_SELECTED: &str =
    "worker requires explicit selected rule applicability";
/// Text `always`.
pub(crate) const ALWAYS: &str = "always";
/// Text `accepted-syntax`.
pub(crate) const ACCEPTED_SYNTAX: &str = "accepted-syntax";
/// Text `applicability`.
pub(crate) const APPLICABILITY: &str = "applicability";
/// Text `worker rule applicability differs fro...`.
pub(crate) const WORKER_RULE_APPLICABILITY_DIFFERS: &str =
    "worker rule applicability differs from the pinned phase policy";
/// Text `python-target=3.7`.
pub(crate) const PYTHON_TARGET_3_7: &str = "python-target=3.7";
/// Text `py37`.
pub(crate) const PY37: &str = "py37";
/// Text `python-target=3.8`.
pub(crate) const PYTHON_TARGET_3_8: &str = "python-target=3.8";
/// Text `py38`.
pub(crate) const PY38: &str = "py38";
/// Text `python-target=3.9`.
pub(crate) const PYTHON_TARGET_3_9: &str = "python-target=3.9";
/// Text `py39`.
pub(crate) const PY39: &str = "py39";
/// Text `python-target=3.10`.
pub(crate) const PYTHON_TARGET_3_10: &str = "python-target=3.10";
/// Text `py310`.
pub(crate) const PY310: &str = "py310";
/// Text `python-target=3.11`.
pub(crate) const PYTHON_TARGET_3_11: &str = "python-target=3.11";
/// Text `py311`.
pub(crate) const PY311: &str = "py311";
/// Text `python-target=3.12`.
pub(crate) const PYTHON_TARGET_3_12: &str = "python-target=3.12";
/// Text `py312`.
pub(crate) const PY312: &str = "py312";
/// Text `python-target=3.13`.
pub(crate) const PYTHON_TARGET_3_13: &str = "python-target=3.13";
/// Text `py313`.
pub(crate) const PY313: &str = "py313";
/// Text `python-target=3.14`.
pub(crate) const PYTHON_TARGET_3_14: &str = "python-target=3.14";
/// Text `py314`.
pub(crate) const PY314: &str = "py314";
/// Text `python-target=3.15`.
pub(crate) const PYTHON_TARGET_3_15: &str = "python-target=3.15";
/// Text `py315`.
pub(crate) const PY315: &str = "py315";
/// Text `unsupported or missing Ruff parser co...`.
pub(crate) const UNSUPPORTED_OR_MISSING_RUFF_PARSER: &str =
    "unsupported or missing Ruff parser configuration";
/// Text `explicit legacy target`.
pub(crate) const EXPLICIT_LEGACY_TARGET: &str = "explicit legacy target";
/// Text `explicit native annotation target`.
pub(crate) const EXPLICIT_NATIVE_ANNOTATION_TARGET: &str = "explicit native annotation target";
/// Text `python-target=latest`.
pub(crate) const PYTHON_TARGET_LATEST: &str = "python-target=latest";
/// Text `python-target=3.6`.
pub(crate) const PYTHON_TARGET_3_6: &str = "python-target=3.6";
/// Text `python-target=3.16`.
pub(crate) const PYTHON_TARGET_3_16: &str = "python-target=3.16";
/// Text `python-target=3.09`.
pub(crate) const PYTHON_TARGET_3_09: &str = "python-target=3.09";
/// Text `python-target=3.10 `.
pub(crate) const PYTHON_TARGET_3_10_2: &str = "python-target=3.10 ";
