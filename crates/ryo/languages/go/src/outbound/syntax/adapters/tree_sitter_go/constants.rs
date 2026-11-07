//! The names of the `tree_sitter_go` adapter: what it says of a file beside what the library says.

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: the language reads a file as UTF-8 text, and the parser reads it no further";

/// What the adapter says of text the parser could not read.
pub(super) const UNREADABLE: &str = "syntax error: the parser could not read this text as go";

/// What the adapter says of a token the parser expected and did not find.
pub(super) const MISSING: &str = "syntax error: a token is missing here";

/// What the adapter says of a statement outside a function.
pub(super) const OUTSIDE_FUNCTION: &str =
    "syntax error: non-declaration statement outside function body";

/// What the adapter says of a list whose last element is followed by a line break and no comma.
pub(super) const NEEDS_COMMA: &str =
    "syntax error: need a trailing comma before the newline that ends the list";

/// What the adapter says of a string whose closing quote the parser did not find.
pub(super) const UNTERMINATED: &str = "syntax error: the string is not terminated";

/// The byte order mark `go/scanner` skips at the start of a file.
pub(super) const BYTE_ORDER_MARK: char = '\u{feff}';

/// The bytes `go/scanner` skips between tokens.
pub(super) const WHITE_SPACE: &[u8] = b" \t\r\n";
