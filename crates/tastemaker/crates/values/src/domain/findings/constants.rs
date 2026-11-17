//! The names of the built-in checks a finding can carry, after `identity#`.

/// A count is not met.
pub const COUNT: &str = "count";
/// A file of a pair is missing.
pub const MISSING: &str = "missing";
/// A file is placed twice, or a spec applied twice at one directory, or a rule reaches a file with two sets of params.
pub const COVERAGE: &str = "coverage";
/// A file does not meet its template.
pub const TEMPLATE: &str = "template";
/// A param has no value.
pub const PARAMS: &str = "params";
/// A file is not placed in a closed directory.
pub const CLOSED: &str = "closed";
/// A rule could not decide.
pub const UNKNOWN: &str = "unknown";
/// An entry's source offers a newer version.
pub const OUTDATED: &str = "outdated";
/// A rule's query is not one ryo can run.
pub const QUERY: &str = "query";
/// Tastemaker's own name: what its own checks are of, beside those of an entry, and what its outputs name it.
pub const TASTEMAKER: &str = "tastemaker";
/// The check every in-source ignore answers to: `tastemaker#ignore`.
pub const IGNORE: &str = "ignore";
/// The check a definition's format answers to, a key Tastemaker folds or a variable problem: `tastemaker#format`.
pub const FORMAT: &str = "format";

/// What joins an entry and one of its checks when a finding of the check is printed: `package/name#count`.
pub const CHECK_SEPARATOR: char = '#';
