//! The names and numbers of the lock's files.

/// The directory of the lock's operations, beneath the product's root.
pub(super) const DIRECTORY: &str = ".asymptote/operations";
/// Why a file of the lock's directory is refused.
pub(super) const UNNAMED: &str = "its name is not a lowercase BLAKE3 digest";
/// The suffix of an operation's temporary file while it is written, beside the operations.
pub(super) const TEMPORARY: &str = ".tmp";
/// The mode a file is created with before the umask, as `std::fs::write` creates one: a temporary file starts private,
/// `0o600`, which a file renamed into place would keep.
#[cfg(unix)]
pub(super) const ORDINARY_MODE: u32 = 0o666;
/// An operation file's extension.
pub(super) const EXTENSION: &str = "lock";
/// The format version every operation states, the one this adapter reads and writes: it stays 1 until the first
/// release, whatever the schema becomes before it.
pub(super) const LOCK_VERSION: u32 = 1;
/// How many bytes of a key's hash its id keeps.
pub(super) const ID_BYTES: usize = 16;
/// What a suppressed key's hash closes with.
pub(super) const SUPPRESSED: &[u8] = b"suppressed";
/// A Windows line ending, which a checkout may give an operation's file.
pub(super) const CRLF: &str = "\r\n";
/// The line ending an operation is addressed with.
pub(super) const LF: &str = "\n";
/// The name of the perf records an operation sets.
pub(super) const PERF: &str = "perf";
