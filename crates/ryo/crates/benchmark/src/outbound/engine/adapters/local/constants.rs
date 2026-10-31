//! SDK boundary protocol and refusals.
pub(super) const NON_UTF8_RULE: &str = "non-UTF8 rule";
pub(super) const QUERY_FILE: &str = "query.ryoql";
pub(super) const DUPLICATE_RULE: &str = "duplicate candidate rule identity";
pub(super) const EMPTY_RULES: &str = "no candidate queries";
pub(super) const EMPTY_CORPUS: &str = "empty candidate corpus";
pub(super) const INVALID_SUBJECT: &str = "finding subject is not a file";
pub(super) const SPAN_COLUMN: &str = "span";
pub(super) const INVALID_SPAN: &str = "finding requires one typed span column";
