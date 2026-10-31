//! The names of the store's files, and the words it writes.

/// The store's file, in the store's directory.
pub(super) const FILE: &str = "check.store";
/// Full failures from the check identified by its build, head and store digest.
pub(super) const DIAGNOSTICS: &str = "check.failures.json";
/// The mode a file is created with before the umask, as `std::fs::write` creates one: a temporary file starts private,
/// `0o600`, which a file renamed into place would keep.
#[cfg(unix)]
pub(super) const ORDINARY_MODE: u32 = 0o666;
/// What marks the directory a cache, for backup tools.
pub(super) const TAG: &str = "CACHEDIR.TAG";
/// What keeps the directory out of version control wherever it is made.
pub(super) const GITIGNORE: &str = ".gitignore";
/// What it says: every file of the directory, itself too.
pub(super) const GITIGNORE_TEXT: &str =
    "# The store of asymptote: rebuilt by any run, never checked in.\n*\n";
/// The tag's text.
pub(super) const TAG_TEXT: &str = "Signature: 8a477f597d28d172789f06886806bc55\n# The store of asymptote: rebuilt by any run, safe to delete.\n";
/// The head of a check run with no commit to name.
pub(super) const DETACHED: &str = "detached";
/// A run that completed in the check that kept it.
pub(super) const COMPLETED: &str = "completed";
/// A run an earlier check completed, kept for a later check to reuse.
pub(super) const RETAINED: &str = "retained";
/// A run that exited with a code that is no verdict.
pub(super) const EXITED: &str = "exited";
/// A run a signal ended.
pub(super) const SIGNALLED: &str = "signalled";
/// A run past its timeout.
pub(super) const TIMED_OUT: &str = "timed-out";
/// A run that could not start.
pub(super) const UNSTARTABLE: &str = "unstartable";
/// A run whose output could not be read.
pub(super) const UNREADABLE: &str = "unreadable";
