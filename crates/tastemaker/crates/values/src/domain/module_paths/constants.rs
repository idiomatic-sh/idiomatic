//! The text a module path is read with as a path.

/// The separator of a path as the matching port reads it.
pub const PATH_SEPARATOR: &str = crate::domain::applications::constants::PATH_SEPARATOR;

/// A module pattern's segment that stands for any number of segments.
pub const ANY_MODULE: &str = "*";

/// The glob segment a module pattern's `*` is read as.
pub const ANY_SEGMENTS: &str = "**";

/// The characters a segment of a module path holds besides letters and digits: whatever else a language writes between
/// two names separates them, so no language's separator is named.
pub const NAME_CHARACTERS: [char; 4] = ['_', '-', '@', '$'];
