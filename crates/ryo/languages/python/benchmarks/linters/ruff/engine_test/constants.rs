//! Literal text of the engine and worker-boundary proofs.
/// Text `LIT`.
pub(crate) const LIT: &str = "LIT";
/// Text `ALT`.
pub(crate) const ALT: &str = "ALT";
/// Text `lazy from __future__ import annotatio...`.
pub(crate) const LAZY_FROM_FUTURE_IMPORT_ANNOTATION: &str =
    "lazy from __future__ import annotations\nvalue=True\n";
/// Text `lazy import collections value=True `.
pub(crate) const LAZY_IMPORT_COLLECTIONS_VALUE_TRUE: &str = "lazy import collections\nvalue=True\n";
/// Text `value=True from __future__ import ann...`.
pub(crate) const VALUE_TRUE_FROM_FUTURE_IMPORT_ANNO: &str =
    "value=True\nfrom __future__ import annotations\n";
/// Text `from __future__ import not_a_feature ...`.
pub(crate) const FROM_FUTURE_IMPORT_NOT_A_FEATURE_V: &str =
    "from __future__ import not_a_feature\nvalue=True\n";
/// Text `rules`.
pub(crate) const RULES: &str = "rules";
/// Text `query.ryoql`.
pub(crate) const QUERY_RYOQL: &str = "query.ryoql";
/// Text `let truth = select id, file, span fro...`.
pub(crate) const LET_TRUTH_SELECT_ID_FILE_SPAN_FROM: &str = "let truth = select id, file, span from \"literal-properties\" where kind = \"boolean\" and boolean = \"true\"; emit truth subject file key(span);";
/// Text `input.py`.
pub(crate) const INPUT_PY: &str = "input.py";
/// Text `python-target=3.15`.
pub(crate) const PYTHON_TARGET_3_15: &str = "python-target=3.15";
/// Text `live selected-rule positive`.
pub(crate) const LIVE_SELECTED_RULE_POSITIVE: &str = "live selected-rule positive";
/// Text `True`.
pub(crate) const TRUE: &str = "True";
/// Text `invalid-syntax`.
pub(crate) const INVALID_SYNTAX: &str = "invalid-syntax";
/// Text `benchmarks/linters/ruff`.
pub(crate) const BENCHMARKS_LINTERS_RUFF: &str = "benchmarks/linters/ruff";
/// Text `benchmark.toml`.
pub(crate) const BENCHMARK_TOML: &str = "benchmark.toml";
/// Text `query-modules`.
pub(crate) const QUERY_MODULES: &str = "query-modules";
/// Text `I001`.
pub(crate) const I001: &str = "I001";
/// Text `RUF100`.
pub(crate) const RUF100: &str = "RUF100";
/// Text `HIDDEN`.
pub(crate) const HIDDEN: &str = "HIDDEN";
/// Text `directive`.
pub(crate) const DIRECTIVE: &str = "directive";
/// Text `file`.
pub(crate) const FILE: &str = "file";
/// Text `code`.
pub(crate) const CODE: &str = "code";
/// Text `noqa_matches`.
pub(crate) const NOQA_MATCHES: &str = "noqa_matches";
/// Text `test/I001`.
pub(crate) const TEST_I001: &str = "test/I001";
/// Text `rules/I001/query.ryoql`.
pub(crate) const RULES_I001_QUERY_RYOQL: &str = "rules/I001/query.ryoql";
/// Text `**`.
pub(crate) const TEXT: &str = "**";
/// Text `test/RUF100`.
pub(crate) const TEST_RUF100: &str = "test/RUF100";
/// Text `rules/RUF100/query.ryoql`.
pub(crate) const RULES_RUF100_QUERY_RYOQL: &str = "rules/RUF100/query.ryoql";
/// Text `suppression_matches`.
pub(crate) const SUPPRESSION_MATCHES: &str = "suppression_matches";
/// Text `let located = select l.directive as i...`.
pub(crate) const LET_LOCATED_SELECT_L_DIRECTIVE_AS: &str = "let located = select l.directive as id, l.file as file, r.span as span from incoming as l join \"source-location\" as r on l.directive = r.id and l.file = r.file; emit located subject file key(id);";
/// Text `test/HIDDEN`.
pub(crate) const TEST_HIDDEN: &str = "test/HIDDEN";
/// Text `incoming`.
pub(crate) const INCOMING: &str = "incoming";
/// Text `config.selected`.
pub(crate) const CONFIG_SELECTED: &str = "config.selected";
/// Text `config.disabled`.
pub(crate) const CONFIG_DISABLED: &str = "config.disabled";
/// Text `config.external`.
pub(crate) const CONFIG_EXTERNAL: &str = "config.external";
/// Text `prefix`.
pub(crate) const PREFIX: &str = "prefix";
/// Text `import local  # noqa: I001 import num...`.
pub(crate) const IMPORT_LOCAL_NOQA_I001_IMPORT_NUMP: &str =
    "import local  # noqa: I001\nimport numpy\nimport unresolved\n";
/// Text `import local import numpy  # noqa: I0...`.
pub(crate) const IMPORT_LOCAL_IMPORT_NUMPY_NOQA_I00: &str =
    "import local\nimport numpy  # noqa: I001\nimport unresolved\n";
/// Text ``.
pub(crate) const EMPTY: &str = "";
/// Text `local.py`.
pub(crate) const LOCAL_PY: &str = "local.py";
/// Text `case.py`.
pub(crate) const CASE_PY: &str = "case.py";
/// Text `broken.py`.
pub(crate) const BROKEN_PY: &str = "broken.py";
/// Text `def broken(  # noqa: I001 `.
pub(crate) const DEF_BROKEN_NOQA_I001: &str = "def broken(  # noqa: I001\n";
/// Text `other.py`.
pub(crate) const OTHER_PY: &str = "other.py";
/// Text `import b import a `.
pub(crate) const IMPORT_B_IMPORT_A: &str = "import b\nimport a\n";
/// Text `import a import b `.
pub(crate) const IMPORT_A_IMPORT_B: &str = "import a\nimport b\n";
/// Text `profile`.
pub(crate) const PROFILE: &str = "profile";
/// Text `RUF200`.
pub(crate) const RUF200: &str = "RUF200";
/// Text `profiles/RUF200/benchmark.toml`.
pub(crate) const PROFILES_RUF200_BENCHMARK_TOML: &str = "profiles/RUF200/benchmark.toml";
/// Text `cases.toml`.
pub(crate) const CASES_TOML: &str = "cases.toml";
/// Text `rules/RUF200`.
pub(crate) const RULES_RUF200: &str = "rules/RUF200";
/// The key of a case's name.
pub(crate) const KEY_NAME: &str = "name";
/// The key of a case's comparison mode.
pub(crate) const KEY_COMPARISON: &str = "comparison";
/// The comparison of the whole report.
pub(crate) const FULL: &str = "full";
/// The key of a case's files.
pub(crate) const KEY_FILES: &str = "files";
/// The key of a case's project context.
pub(crate) const KEY_CONTEXT: &str = "context";
/// The key of the context's logical root.
pub(crate) const KEY_LOGICAL_ROOT: &str = "logical-root";
/// The logical root of a project.
pub(crate) const PROJECT: &str = "project";
/// The key of a case's expected findings.
pub(crate) const KEY_EXPECTED: &str = "expected";
/// The key of a finding's rule.
pub(crate) const KEY_RULE: &str = "rule";
/// The key of a finding's file.
pub(crate) const KEY_FILE: &str = "file";
/// The key of a finding's start offset.
pub(crate) const KEY_START: &str = "start";
/// The key of a finding's end offset.
pub(crate) const KEY_END: &str = "end";
/// The key of the cases of a case file.
pub(crate) const KEY_CASES: &str = "cases";
/// A file that is TOML by its name and holds Python.
pub(crate) const OTHER_TOML: &str = "other.toml";
/// Python that does not parse.
pub(crate) const BROKEN_SOURCE: &str = "def broken(\n";
/// The offset at which the broken source is refused.
pub(crate) const BROKEN_OFFSET: i64 = 12;
/// Text `cases`.
pub(crate) const CASES: &str = "cases";
/// Text `invalid-toml-and-invalid-python`.
pub(crate) const INVALID_TOML_AND_INVALID_PYTHON: &str = "invalid-toml-and-invalid-python";
/// Text `other-toml-python-source-without-mani...`.
pub(crate) const OTHER_TOML_PYTHON_SOURCE_WITHOUT_M: &str =
    "other-toml-python-source-without-manifest-diagnostic";
/// Text `unknown-extension-still-python`.
pub(crate) const UNKNOWN_EXTENSION_STILL_PYTHON: &str = "unknown-extension-still-python";
/// Text `explicit-python-unrelated-toml-still-...`.
pub(crate) const EXPLICIT_PYTHON_UNRELATED_TOML_STI: &str =
    "explicit-python-unrelated-toml-still-parses";
/// Text `always`.
pub(crate) const ALWAYS: &str = "always";
/// Text `python-target=3.10`.
pub(crate) const PYTHON_TARGET_3_10: &str = "python-target=3.10";
/// Text `python`.
pub(crate) const PYTHON: &str = "python";
/// Text `run-ryo`.
pub(crate) const RUN_RYO: &str = "run-ryo";
/// Text `profile-ryo`.
pub(crate) const PROFILE_RYO: &str = "profile-ryo";
/// Text `diagnostic-ryo`.
pub(crate) const DIAGNOSTIC_RYO: &str = "diagnostic-ryo";
/// Text `diagnostic-unknown`.
pub(crate) const DIAGNOSTIC_UNKNOWN: &str = "diagnostic-unknown";
/// Text `request.json`.
pub(crate) const REQUEST_JSON: &str = "request.json";
/// Text `{"root":"/tmp/project","files":["a.py"]}`.
pub(crate) const ROOT_TMP_PROJECT_FILES_A_PY: &str = r#"{"root":"/tmp/project","files":["a.py"]}"#;
/// Text `--request-file`.
pub(crate) const REQUEST_FILE: &str = "--request-file";
/// Text `python-target=3.11`.
pub(crate) const PYTHON_TARGET_3_11: &str = "python-target=3.11";
/// Text `python-target=3.9`.
pub(crate) const PYTHON_TARGET_3_9: &str = "python-target=3.9";
/// Text `python-target=3.7`.
pub(crate) const PYTHON_TARGET_3_7: &str = "python-target=3.7";
/// Text `python-target=3.8`.
pub(crate) const PYTHON_TARGET_3_8: &str = "python-target=3.8";
/// Text `python-target=3.12`.
pub(crate) const PYTHON_TARGET_3_12: &str = "python-target=3.12";
/// Text `python-target=3.13`.
pub(crate) const PYTHON_TARGET_3_13: &str = "python-target=3.13";
/// Text `python-target=3.14`.
pub(crate) const PYTHON_TARGET_3_14: &str = "python-target=3.14";
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
