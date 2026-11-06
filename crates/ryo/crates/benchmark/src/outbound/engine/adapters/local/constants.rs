//! SDK boundary protocol and refusals.
/// Refuse rule directory names outside the query identifier encoding.
pub(super) const NON_UTF8_RULE: &str = "non-UTF8 rule";
/// Canonical colocated query filename.
pub(super) const QUERY_FILE: &str = "query.ryoql";
/// Refuse repeated rule identities rather than overwriting queries.
pub(super) const DUPLICATE_RULE: &str = "duplicate candidate rule identity";
/// A candidate run must evaluate at least one authored query.
pub(super) const EMPTY_RULES: &str = "no candidate queries";
/// A candidate run must receive at least one checked source.
pub(super) const EMPTY_CORPUS: &str = "empty candidate corpus";
/// Benchmark findings must identify a source file subject.
pub(super) const INVALID_SUBJECT: &str = "finding subject is not a file";
/// Typed source range column required from every authored rule.
pub(super) const SPAN_COLUMN: &str = "span";
/// Refuse absent, duplicate or untyped diagnostic source ranges.
pub(super) const INVALID_SPAN: &str = "finding requires one typed span column";
/// Text of the always in this part.
pub(super) const ALWAYS: &str = "always";
/// Text of the accepted syntax in this part.
pub(super) const ACCEPTED_SYNTAX: &str = "accepted-syntax";
/// Text of the applicability map does not exactly match in this part.
pub(super) const APPLICABILITY_MAP_DOES_NOT_EXACTLY_MATCH: &str = "applicability map does not exactly match selected candidate rules";
/// Text of the applicability skips require frontend diagnos in this part.
pub(super) const APPLICABILITY_SKIPS_REQUIRE_FRONTEND_DIAGNOS: &str = "applicability skips require frontend diagnostic projection";
/// Text of the skipped rule pairs exceed the configured in this part.
pub(super) const SKIPPED_RULE_PAIRS_EXCEED_THE_CONFIGURED: &str = "skipped rule pairs exceed the configured selection";
/// Text of the rules directory has no profile root in this part.
pub(super) const RULES_DIRECTORY_HAS_NO_PROFILE_ROOT: &str = "rules directory has no profile root";
/// Text of the module source in this part.
pub(super) const MODULE_SOURCE: &str = "module source";
/// Text of the worker context mode requires a matching in this part.
pub(super) const WORKER_CONTEXT_MODE_REQUIRES_A_MATCHING: &str = "worker context mode requires a matching explicit manifest";
/// Text of the explicit primary role provider is missing in this part.
pub(super) const EXPLICIT_PRIMARY_ROLE_PROVIDER_IS_MISSING: &str = "explicit primary-role provider is missing or ambiguous";
/// Text of the ryo diagnostic rule file in this part.
pub(super) const RYO_DIAGNOSTIC_RULE_FILE: &str = "ryo::diagnostic::rule_file";
/// Text of the ryo diagnostic compiled prefix in this part.
pub(super) const RYO_DIAGNOSTIC_COMPILED_PREFIX: &str = "ryo::diagnostic::compiled_prefix";
/// Text of the ryo diagnostic compiled subplan in this part.
pub(super) const RYO_DIAGNOSTIC_COMPILED_SUBPLAN: &str = "ryo::diagnostic::compiled_subplan";
/// Text of the ryo diagnostic subplan cut in this part.
pub(super) const RYO_DIAGNOSTIC_SUBPLAN_CUT: &str = "ryo::diagnostic::subplan_cut";
/// Text of the ryo diagnostic stored join in this part.
pub(super) const RYO_DIAGNOSTIC_STORED_JOIN: &str = "ryo::diagnostic::stored_join";
/// Text of the ryo diagnostic rule step in this part.
pub(super) const RYO_DIAGNOSTIC_RULE_STEP: &str = "ryo::diagnostic::rule_step";
/// Text of the ryo diagnostic execution in this part.
pub(super) const RYO_DIAGNOSTIC_EXECUTION: &str = "ryo::diagnostic::execution";
/// Text of the ryo diagnostic selection in this part.
pub(super) const RYO_DIAGNOSTIC_SELECTION: &str = "ryo::diagnostic::selection";
/// Text of the ryo diagnostic selection detail in this part.
pub(super) const RYO_DIAGNOSTIC_SELECTION_DETAIL: &str = "ryo::diagnostic::selection_detail";
/// Text of the ryo diagnostic working set in this part.
pub(super) const RYO_DIAGNOSTIC_WORKING_SET: &str = "ryo::diagnostic::working_set";
/// Text of the ryo diagnostic layout in this part.
pub(super) const RYO_DIAGNOSTIC_LAYOUT: &str = "ryo::diagnostic::layout";
/// Text of the ryo diagnostic graph in this part.
pub(super) const RYO_DIAGNOSTIC_GRAPH: &str = "ryo::diagnostic::graph";
/// Text of the ryo diagnostic file program in this part.
pub(super) const RYO_DIAGNOSTIC_FILE_PROGRAM: &str = "ryo::diagnostic::file_program";
/// Text of the ryo diagnostic resource routes in this part.
pub(super) const RYO_DIAGNOSTIC_RESOURCE_ROUTES: &str = "ryo::diagnostic::resource_routes";
/// Text of the ryo diagnostic lifecycle memory in this part.
pub(super) const RYO_DIAGNOSTIC_LIFECYCLE_MEMORY: &str = "ryo::diagnostic::lifecycle_memory";
/// Text of the ryo diagnostic row progress in this part.
pub(super) const RYO_DIAGNOSTIC_ROW_PROGRESS: &str = "ryo::diagnostic::row_progress";
/// Text of the profile root in this part.
pub(super) const PROFILE_ROOT: &str = "profile root";
/// Text of the rules in this part.
pub(super) const RULES: &str = "rules";
/// Text of the f401 in this part.
pub(super) const F401: &str = "F401";
/// Text of the rule directory in this part.
pub(super) const RULE_DIRECTORY: &str = "rule directory";
/// Text of the f401 query ryoql in this part.
pub(super) const F401_QUERY_RYOQL: &str = "F401/query.ryoql";
/// Text of the import shared from python ruff suppress in this part.
pub(super) const IMPORT_SHARED_FROM_PYTHON_RUFF_SUPPRESS: &str = "import shared from \"python/ruff/suppress\";";
/// Text of the query source in this part.
pub(super) const QUERY_SOURCE: &str = "query source";
/// Text of the fragments suppress ryoql in this part.
pub(super) const FRAGMENTS_SUPPRESS_RYOQL: &str = "fragments/suppress.ryoql";
/// Text of the module parent in this part.
pub(super) const MODULE_PARENT: &str = "module parent";
/// Text of the module folder in this part.
pub(super) const MODULE_FOLDER: &str = "module folder";
/// Text of the input source relation id node in this part.
pub(super) const INPUT_SOURCE_RELATION_ID_NODE: &str = "input source: relation<id: node>;\n";
/// Text of the python ruff suppress in this part.
pub(super) const PYTHON_RUFF_SUPPRESS: &str = "python/ruff/suppress";
/// Text of the module bundle is loaded in the in this part.
pub(super) const MODULE_BUNDLE_IS_LOADED_IN_THE: &str = "module bundle is loaded in the worker";
/// Text of the rule in this part.
pub(super) const RULE: &str = "rule";
/// Text of the complete applicability map loads in this part.
pub(super) const COMPLETE_APPLICABILITY_MAP_LOADS: &str = "complete applicability map loads";
/// Text of the f402 in this part.
pub(super) const F402: &str = "F402";
/// Text of the extra applicability entry is refused in this part.
pub(super) const EXTRA_APPLICABILITY_ENTRY_IS_REFUSED: &str = "extra applicability entry is refused";
/// Text of the does not exactly match in this part.
pub(super) const DOES_NOT_EXACTLY_MATCH: &str = "does not exactly match";
/// Text of the parse only in this part.
pub(super) const PARSE_ONLY: &str = "parse-only";
/// Text of the unknown applicability is refused in this part.
pub(super) const UNKNOWN_APPLICABILITY_IS_REFUSED: &str = "unknown applicability is refused";
/// Text of the unsupported applicability in this part.
pub(super) const UNSUPPORTED_APPLICABILITY: &str = "unsupported applicability";
/// Text of the input py in this part.
pub(super) const INPUT_PY: &str = "input.py";
/// Text of the expected success in this part.
pub(super) const EXPECTED_SUCCESS: &str = "expected success";
/// Text of the expected refusal in this part.
pub(super) const EXPECTED_REFUSAL: &str = "expected refusal";
/// Text of the else py in this part.
pub(super) const ELSE_PY: &str = "else.py";
/// Text of the outside ryoql in this part.
pub(super) const OUTSIDE_RYOQL: &str = "../outside.ryoql";
/// Text of the traversal is refused in this part.
pub(super) const TRAVERSAL_IS_REFUSED: &str = "traversal is refused";
/// Text of the unsafe corpus path in this part.
pub(super) const UNSAFE_CORPUS_PATH: &str = "unsafe corpus path";
/// Text of the fragments in this part.
pub(super) const FRAGMENTS: &str = "fragments";
/// Text of the outside ryoql 2 in this part.
pub(super) const OUTSIDE_RYOQL_2: &str = "outside.ryoql";
/// Text of the module bytes in this part.
pub(super) const MODULE_BYTES: &str = "module bytes";
/// Text of the symlink fixture in this part.
pub(super) const SYMLINK_FIXTURE: &str = "symlink fixture";
/// Text of the symlink is refused in this part.
pub(super) const SYMLINK_IS_REFUSED: &str = "symlink is refused";
/// Text of the query module is a symlink in this part.
pub(super) const QUERY_MODULE_IS_A_SYMLINK: &str = "query module is a symlink";
/// Text of the published in this part.
pub(super) const PUBLISHED: &str = "published";
/// Text of the file in this part.
pub(super) const FILE: &str = "file";
/// Text of the b002 in this part.
pub(super) const B002: &str = "B002";
/// Text of the matched in this part.
pub(super) const MATCHED: &str = "matched";
/// Text of the typo in this part.
pub(super) const TYPO: &str = "typo";
/// Text of the unregistered consumer in this part.
pub(super) const UNREGISTERED_CONSUMER: &str = "unregistered consumer";
