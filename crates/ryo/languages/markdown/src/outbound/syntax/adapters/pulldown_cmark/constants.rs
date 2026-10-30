//! What the `pulldown_cmark` adapter reads with.

use pulldown_cmark::Options;

/// The extensions the language reads with: GitHub Flavored Markdown (tables, footnotes, strikethrough, task lists, alerts), math as
/// GitHub writes it, and front matter. Smart punctuation is off: it rewrites the text the events carry. The extensions GitHub does
/// not have (heading attributes, definition lists, superscript, subscript, wiki links) are off, so their syntax is text, as GitHub
/// shows it.
pub(super) const OPTIONS: Options = Options::ENABLE_TABLES
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_GFM)
    .union(Options::ENABLE_MATH)
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS);

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str =
    "invalid UTF-8: a Markdown file is UTF-8 text, and the parser reads it no further";

/// The bytes that are white space between the events of the parser.
pub(super) const WHITE_SPACE: &[u8] = b" \t\r\n";

/// The depth of a heading, as the text the adapter carries on its marker, from one to six.
pub(super) const DEPTHS: [&str; 6] = ["1", "2", "3", "4", "5", "6"];
