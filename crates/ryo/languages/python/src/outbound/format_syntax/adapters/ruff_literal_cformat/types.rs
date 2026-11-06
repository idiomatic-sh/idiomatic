//! The cformat grammar's cursor and result types.

use crate::domain::format_syntax::types::{FormatFieldValue, FormatParseError};

/// One Unicode scalar and its position in the decoded string.
#[derive(Clone, Copy)]
pub(super) struct Character {
    /// Zero-based Unicode scalar index.
    pub index: usize,
    /// The decoded scalar.
    pub value: char,
}

/// A scalar iterator with one character of lookahead.
pub(super) struct Cursor<'a> {
    /// Remaining scalar positions and values.
    pub characters: std::iter::Enumerate<std::str::Chars<'a>>,
    /// The lookahead value, if already read.
    pub peeked: Option<Character>,
}

/// The grammar fields retained from one conversion specification.
pub(super) struct ParsedField {
    /// Argument consumption and conversion metadata.
    pub field: FormatFieldValue,
}

/// Syntax failure or inability to retain a complete result.
pub(super) enum Failure {
    /// A concrete grammar diagnostic.
    Malformed(FormatParseError),
    /// The input could not be retained completely.
    Unavailable,
}
