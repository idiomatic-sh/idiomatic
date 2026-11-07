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
pub(super) const ALWAYS: &str = ryo_benchmark_contracts::domain::requests::constants::ALWAYS;
/// Text of the accepted syntax in this part.
pub(super) const ACCEPTED_SYNTAX: &str =
    ryo_benchmark_contracts::domain::requests::constants::ACCEPTED_SYNTAX;
/// Text of the applicability map does not exactly match in this part.
pub(super) const APPLICABILITY_MAP_DOES_NOT_EXACTLY_MATCH: &str =
    "applicability map does not exactly match selected candidate rules";
/// Text of the applicability skips require frontend diagnos in this part.
pub(super) const APPLICABILITY_SKIPS_REQUIRE_FRONTEND_DIAGNOS: &str =
    "applicability skips require frontend diagnostic projection";
/// Text of the skipped rule pairs exceed the configured in this part.
pub(super) const SKIPPED_RULE_PAIRS_EXCEED_THE_CONFIGURED: &str =
    "skipped rule pairs exceed the configured selection";
/// Text of the rules directory has no profile root in this part.
pub(super) const RULES_DIRECTORY_HAS_NO_PROFILE_ROOT: &str = "rules directory has no profile root";
/// Text of the module source in this part.
pub(super) const MODULE_SOURCE: &str = "module source";
/// Text of the worker context mode requires a matching in this part.
pub(super) const WORKER_CONTEXT_MODE_REQUIRES_A_MATCHING: &str =
    "worker context mode requires a matching explicit manifest";
/// Text of the explicit primary role provider is missing in this part.
pub(super) const EXPLICIT_PRIMARY_ROLE_PROVIDER_IS_MISSING: &str =
    "explicit primary-role provider is missing or ambiguous";
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
/// Refuse a requested path repeated in the source inventory.
pub(super) const DUPLICATE_SOURCE: &str = "duplicate input path";
