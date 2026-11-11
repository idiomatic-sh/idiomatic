//! The constants of the part: the text its code uses.

pub(super) const OPEN_OBJECT: char = '{';
pub(super) const CLOSE_OBJECT: char = '}';
pub(super) const OPEN_ARRAY: char = '[';
pub(super) const CLOSE_ARRAY: char = ']';
pub(super) const SEPARATOR: char = ',';
pub(super) const ASSIGN: char = ':';
pub(super) const QUOTE: char = '"';
pub(super) const BACKSLASH: char = '\\';
pub(super) const NULL: &str = "null";
pub(super) const TRUE: &str = "true";
pub(super) const FALSE: &str = "false";
pub(super) const ESCAPED_QUOTE: &str = "\\\"";
pub(super) const ESCAPED_BACKSLASH: &str = "\\\\";
pub(super) const ESCAPED_NEWLINE: &str = "\\n";
pub(super) const ESCAPED_RETURN: &str = "\\r";
pub(super) const ESCAPED_TAB: &str = "\\t";
pub(super) const ESCAPE_UNICODE: &str = "\\u";
pub(super) const NEWLINE: char = '\n';
pub(super) const RETURN: char = '\r';
pub(super) const TAB: char = '\t';
pub(super) const HEX_RADIX: u32 = 16;
pub(super) const HEX_DIGITS: u32 = 4;
pub(super) const HEX_NIBBLE_BITS: u32 = 4;
pub(super) const HEX_NIBBLE_MASK: u32 = 0xF;
