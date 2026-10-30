//! The text the `captures` concept reads and writes captures with.

/// What opens a capture: the `{` of `{name}`.
pub const OPEN: char = '{';
/// What closes a capture: the `}` of `{name}`.
pub const CLOSE: char = '}';
/// What separates the segments of a path, and the values an expanded tool's id is written with.
pub const SEPARATOR: char = '/';
/// What opens a shell's expansion before a `{`: `${HOME}` is text, never a capture.
pub const EXPANSION: char = '$';
