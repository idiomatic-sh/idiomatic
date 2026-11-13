//! The punctuation and escapes of a canonical document.
pub(super) const NULL: &str = "null";
pub(super) const QUOTE: &str = "\"";
pub(super) const COMMA: &str = ",";
pub(super) const COLON: &str = ":";
pub(super) const OPEN_LIST: &str = "[";
pub(super) const CLOSE_LIST: &str = "]";
pub(super) const OPEN_OBJECT: &str = "{";
pub(super) const CLOSE_OBJECT: &str = "}";
pub(super) const ESCAPED_QUOTE: &str = "\\\"";
pub(super) const ESCAPED_BACKSLASH: &str = "\\\\";
pub(super) const ESCAPED_NEWLINE: &str = "\\n";
pub(super) const ESCAPED_RETURN: &str = "\\r";
pub(super) const ESCAPED_TAB: &str = "\\t";
pub(super) const ESCAPED_BACKSPACE: &str = "\\b";
pub(super) const ESCAPED_FORM_FEED: &str = "\\f";
/// The key under which a canonical document names a file.
pub(crate) const FILE: &str = "file";
