//! The names the `toml` adapter reads a manifest with.

/// The source of a dependency that names no directory, git repository or registry, as `[patch]` names it.
pub(super) const CRATES_IO: &str = "crates-io";

/// What separates a package's name from its version in a `[replace]` key.
pub(super) const REPLACED_VERSION: char = ':';
