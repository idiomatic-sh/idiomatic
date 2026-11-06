//! The pinned tool's process protocol and boundary refusals.

/// Run Ruff lint diagnostics.
pub(crate) const CHECK: &str = "check";
/// Disable ambient project and user configuration.
pub(crate) const ISOLATED: &str = "--isolated";
/// Ordered import search roots used by the pinned isolated Ruff invocation.
pub(crate) const ISOLATED_PROJECT_ROOTS: [&str; 2] = ["", "src"];
/// Refusal when an explicit host project scope differs from Ruff's isolated scope.
pub(crate) const UNSUPPORTED_PROJECT_ROOTS: &str =
    "reference profile cannot match the declared project search roots";
/// Select the diagnostic wire representation.
pub(crate) const OUTPUT_FORMAT: &str = "--output-format";
/// Machine-readable Ruff diagnostic format.
pub(crate) const JSON: &str = "json";
/// Supply an explicit retained reference cache directory.
pub(crate) const CACHE_DIR: &str = "--cache-dir";
/// Reference-specific state directory below one observation.
pub(crate) const CACHE_NAME: &str = "ruff";
/// Separate explicit input paths from options.
pub(crate) const END_OPTIONS: &str = "--";
/// Ask Ruff to enumerate its selected source files.
pub(crate) const SHOW_FILES: &str = "--show-files";
/// Discovery root relative to the explicit working directory.
pub(crate) const CURRENT_DIRECTORY: &str = ".";
/// Environment control for Rayon worker count.
pub(crate) const WORKERS_ENV: &str = "RAYON_NUM_THREADS";
/// Pin the reference to one worker for comparable execution.
pub(crate) const ONE_WORKER: &str = "1";
/// Refusal preventing an empty explicit selection from becoming directory discovery.
pub(crate) const EMPTY_INPUT: &str = "empty input list would make Ruff walk the directory";
/// Refusal for duplicated requested source paths.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate requested source file";
/// Refusal when discovery escapes the corpus root.
pub(crate) const DISCOVERED_OUTSIDE: &str = "discovered file outside corpus";
/// Refusal for paths outside the text-based benchmark contract.
pub(crate) const NON_UTF8_PATH: &str = "non-UTF8 corpus path";
/// Refusal when discovery supplies no source files.
pub(crate) const EMPTY_DISCOVERY: &str = "empty discovered corpus";
/// Select the exact compared rule set.
pub(crate) const SELECT: &str = "--select";
/// Pin syntax and target-gated diagnostics to the prepared Python 3.10 oracle.
pub(crate) const TARGET_VERSION: &str = "--target-version";
/// Separator in the reference rule-selection argument.
pub(crate) const RULE_SEPARATOR: &str = ",";
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
/// Stable command, artifact or observation identity for rules directory.
pub(crate) const RULES_DIRECTORY: &str = "rules";
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
/// The phases of the pinned Ruff rules, as pinned beside the profiles.
pub(crate) const PINNED_RULE_PHASES: &str = include_str!("../profiles/pinned-rule-phases.json");
