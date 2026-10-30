//! The name of the store's file, and what its directory holds besides.

/// The file this use case's store is kept in, in the store's directory.
pub(super) const FILE: &str = "check.store";
/// What the store's directory holds besides its files: a note that it is a cache, for backup tools.
pub(super) const TAG: &str = crate::domain::definitions::constants::TAG;
/// What keeps the store out of version control wherever it is made, as a tool's cache says of itself.
pub(super) const GITIGNORE: &str = crate::domain::definitions::constants::GITIGNORE;
/// What it says: every file of the directory, itself too.
pub(super) const GITIGNORE_TEXT: &str = crate::domain::definitions::constants::GITIGNORE_TEXT;
/// The note.
pub(super) const TAG_TEXT: &str = crate::domain::definitions::constants::TAG_TEXT;
/// The mode a file made by `std::fs::write` starts with, before the umask takes its part.
#[cfg(unix)]
pub(super) const ORDINARY_MODE: u32 = 0o666;
