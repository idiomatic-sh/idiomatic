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
/// Query all generic frontend diagnostics as an internal benchmark report rule.
pub(super) const INVALID_SYNTAX_QUERY: &str = "let diagnostics = select id, file, span from \"syntax-error\";\nemit diagnostics subject file key(id);\n";
