//! The names the predicate service's tests give text.

/// The text of the passing comparison.
pub(super) const PASS: &str = "pass";
/// The text of the stopping comparison.
pub(super) const STOP: &str = "stop";
/// The subject text the invalid pattern is matched against.
pub(super) const SUBJECT: &str = "subject";
/// A pattern that is not a valid regular expression.
pub(super) const INVALID_PATTERN: &str = "[";
/// A source with no bytes.
pub(super) const EMPTY_SOURCE: &[u8] = b"";
