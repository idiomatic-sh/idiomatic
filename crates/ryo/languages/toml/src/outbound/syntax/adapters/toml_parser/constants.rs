//! The numbers the `toml_parser` adapter reads with.

use ryo::languages::Count;

/// How deep arrays and inline tables may nest before the parser is told to stop: the limit the `toml` crate reads with, so that the
/// tree and the library's diagnostic stop at the same place.
pub(super) const NESTING: Count = Count(80);

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: a TOML file is UTF-8 text, and the parser reads it no further";

/// What TOML writes between the digits of a number to group them, which a number's value leaves out.
pub(super) const DIGIT_SEPARATOR: char = '_';
