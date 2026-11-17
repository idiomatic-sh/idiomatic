//! The text the `captures` concept writes captures with.

/// What stands between two captures when a message writes them.
pub const CAPTURE_SEPARATOR: &str = ", ";

/// What makes the character after it in a path pattern stand for itself: `\\*` is a star, never a wildcard.
pub const PATTERN_ESCAPE: char = '\\';
/// The characters a path pattern gives a meaning: a wildcard, a capture's braces and the escape itself.
pub const PATTERN_SPECIALS: [char; 5] = ['*', '?', '{', '}', PATTERN_ESCAPE];
/// What opens a capture's name in a path pattern.
pub const CAPTURE_OPEN: char = '{';
/// What closes a capture's name in a path pattern.
pub const CAPTURE_CLOSE: char = '}';
/// What marks a capture of whole segments: `{*name}`.
pub const SPAN_MARK: char = '*';
/// What marks a capture whose value names a directory beside the file: `{name:dir}`.
pub const DIRECTORY_MARK: &str = ":dir";
/// What opens a mark after a capture's name: a name holds none, so `{*name:dir}` and `{name:dirs}` are refused.
pub const MARK_OPEN: char = ':';
/// A path pattern's wildcard of any text within a segment.
pub const STAR: char = '*';
/// A path pattern's wildcard of one character.
pub const QUESTION: char = '?';
