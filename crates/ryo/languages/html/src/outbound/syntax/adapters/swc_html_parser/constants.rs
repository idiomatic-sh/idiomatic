//! The text the `swc_html_parser` adapter reads with.

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: the language reads a file as UTF-8 text, and the parser reads it no further";

/// The ASCII white space HTML strips from both ends of a URL-valued attribute before it reads the URL.
pub(super) const HTML_WHITESPACE: [char; 5] = ['\t', '\n', '\u{c}', '\r', ' '];
