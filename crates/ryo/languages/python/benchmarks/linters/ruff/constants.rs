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
/// Pin the reference to one worker for comparable execution.
pub(crate) const ONE_WORKER: &str = "1";
/// Refusal preventing an empty explicit selection from becoming directory discovery.
pub(crate) const EMPTY_INPUT: &str = "empty input list would make Ruff walk the directory";
/// Refusal for duplicated requested source paths.
pub(crate) const DUPLICATE_INPUT: &str = "duplicate requested source file";
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
/// Text `worker requires an explicit supported...`.
pub(crate) const WORKER_REQUIRES_AN_EXPLICIT_SUPPOR: &str =
    "worker requires an explicit supported parser configuration";
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
/// The phases of the pinned Ruff rules, as pinned beside the profiles.
pub(crate) const PINNED_RULE_PHASES: &str = include_str!("profiles/pinned-rule-phases.json");
/// The arguments that make the pinned reference list the files it selects by default, one path per line.
pub(crate) const DISCOVERY_ARGUMENTS: [&str; 4] = [CHECK, ISOLATED, SHOW_FILES, CURRENT_DIRECTORY];
