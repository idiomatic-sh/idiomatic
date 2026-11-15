//! The text and the kinds the `ra_ap_syntax` adapter's code uses.

use ra_ap_syntax::SyntaxKind;

/// The macro whose arguments are the elements of an array, or an element and its count.
pub(super) const VEC: &str = "vec";

/// What an argument list read as one parenthesized expression parses to: a tuple, or the parentheses around one argument.
pub(super) const LIST_ROOTS: [SyntaxKind; 2] = [SyntaxKind::TUPLE_EXPR, SyntaxKind::PAREN_EXPR];

/// What `vec!`'s arguments read as one bracketed expression parse to.
pub(super) const ARRAY_ROOTS: [SyntaxKind; 1] = [SyntaxKind::ARRAY_EXPR];

/// The macro whose arguments after the first are a pattern.
pub(super) const MATCHES: &str = "matches";

/// A carriage return and a line feed, which a raw string reads as one line feed.
pub(super) const CRLF: &str = "\r\n";

/// A line feed.
pub(super) const LF: &str = "\n";

/// What a byte that is not part of a character is read as: one byte the lexer reports, so the offsets of the rest do not move.
pub(super) const UNREADABLE: char = '\u{1}';
