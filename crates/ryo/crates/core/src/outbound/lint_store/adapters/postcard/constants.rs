//! The names of the postcard store adapter: the text of the places it writes.

/// The file of the store in the cache directory.
pub(super) const FILE: &str = "lint.store";

/// What a file is called while it is written, before it is renamed into place.
pub(super) const TEMPORARY: &str = "lint.store.tmp";

/// What the store says when its place holds something that is not a file.
pub(super) const NOT_A_FILE: &str = "the store is not a file";

/// What the store says when its table of contents names a section that does not end inside the file.
pub(super) const OUTSIDE: &str = "the table of contents names a section past the end of the store";

/// What the store says when a section a run asks for does not read back.
pub(super) const UNDECODED: &str = "a section of the store does not read back";

/// How long the number that says how long the table of contents is, immediately before the checked format trailer.
pub(super) const LENGTH: u64 = 8;

/// Checked before positional postcard decoding; old stores become ordinary cache misses.
pub(super) const FORMAT: &[u8] = b"ryo-lint-store\0permissions-v1\0observation-readiness-v2\0";
