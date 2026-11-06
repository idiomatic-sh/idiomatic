//! Runner command and artifact protocol.
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
