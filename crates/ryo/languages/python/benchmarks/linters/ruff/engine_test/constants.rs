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
/// Text `benchmark.toml`.
pub(crate) const BENCHMARK_TOML: &str = "benchmark.toml";
/// Text `case.py`.
pub(crate) const CASE_PY: &str = "case.py";
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
/// Text `a.py`.
pub(crate) const A_PY: &str = "a.py";
/// Text `b.py`.
pub(crate) const B_PY: &str = "b.py";
/// Text `accepted-syntax`: the applicability policy under which a rule is skipped on a file the parser rejects.
pub(crate) const ACCEPTED_SYNTAX: &str = "accepted-syntax";
/// The rule an equivalence fixture is staged as.
pub(crate) const ROOTED001: &str = "ROOTED001";
/// The program whose six names a rooted traversal reaches from two roots sharing each identity.
pub(crate) const SPARSE_PIE794_RELEVANT: &str = "value = outer(deep(leaf(a)), shallow(b))\n";
/// The relevant program with an await inside a root's descendants.
pub(crate) const SPARSE_NONEMPTY_AWAIT: &str =
    "async def sample():\n    value = outer(deep(leaf(await a())), shallow(b))\n";
/// An await outside every root.
pub(crate) const SPARSE_UNRELATED_AWAIT: &str = "async def unrelated():\n    await work()\n";
/// The PLR1708 program: a generator body whose nested functions, lambda, annotation and class are scope boundaries.
pub(crate) const SPARSE_PLR1708_RELEVANT: &str = "def f():\n    yield outer(deep(1))\n    def inner():\n        yield 2\n        raise StopIteration\n    callback = lambda: (yield 3)\n    annotation: \"StopIteration\"\n    class C:\n        raise StopIteration\n    raise StopIteration\n";
