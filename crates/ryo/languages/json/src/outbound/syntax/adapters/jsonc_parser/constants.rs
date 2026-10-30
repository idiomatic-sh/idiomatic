//! The text the `jsonc_parser` adapter reads with.

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: the language reads a file as UTF-8 text, and the parser reads it no further";

/// What the adapter says of a comment in a file read as json as the standard defines it.
pub(super) const COMMENT_IN_JSON: &str =
    "comments are not allowed in JSON (RFC 8259): only a file read as json with comments has them";

/// What the adapter says of a comma before a closing brace or bracket in a file read as json as the standard defines it.
pub(super) const TRAILING_COMMA_IN_JSON: &str = "a trailing comma is not allowed in JSON (RFC 8259): only a file read as json with comments has one";

/// What the adapter says of a file with no value in it, read as json as the standard defines it.
pub(super) const NO_VALUE: &str = "expected a value: a JSON text is one value (RFC 8259)";

/// What the adapter says of a control character written in a string: the standard allows none unescaped, and the library reads one.
pub(super) const CONTROL_IN_STRING: &str =
    "a control character in a string must be escaped (RFC 8259, section 7)";

/// What the adapter says of text between the tokens of a file the library read that is no token of JSON.
pub(super) const UNREAD_TEXT: &str =
    "text that is no token of JSON (RFC 8259): only white space may stand between its tokens";
