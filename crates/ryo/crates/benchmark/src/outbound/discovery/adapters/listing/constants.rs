//! Names owned by the listing adapter.
/// The refusal when the command lists a file that is not under the worktree root.
pub(super) const DISCOVERED_OUTSIDE: &str = "discovered file outside corpus";
/// The refusal when the command lists a path that is not text.
pub(super) const NON_UTF8_PATH: &str = "non-UTF8 corpus path";
/// The refusal when the command lists no file.
pub(super) const EMPTY_DISCOVERY: &str = "empty discovered corpus";
