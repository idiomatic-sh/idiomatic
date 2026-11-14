//! The text the `ra_ap_syntax` adapter's code uses.

/// The macro whose arguments are also separated by a semicolon and are not named.
pub(super) const VEC: &str = "vec";

/// The macro whose arguments after the first are a pattern.
pub(super) const MATCHES: &str = "matches";

/// A carriage return and a line feed, which a raw string reads as one line feed.
pub(super) const CRLF: &str = "\r\n";

/// A line feed.
pub(super) const LF: &str = "\n";

/// What a byte that is not part of a character is read as: one byte the lexer reports, so the offsets of the rest do not move.
pub(super) const UNREADABLE: char = '\u{1}';
