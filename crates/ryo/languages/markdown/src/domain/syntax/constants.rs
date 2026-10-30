//! The names of the `syntax` concept: the delimiters of an HTML comment, and the reasons no relation models a kind of the trees,
//! which `sample` gives.

/// What opens an HTML comment.
pub(crate) const COMMENT_OPEN: &str = "<!--";

/// What closes an HTML comment.
pub(crate) const COMMENT_CLOSE: &str = "-->";

/// A block or a span of the document's structure.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STRUCTURE: &str = "a block or a span of the document no relation models beyond its place in the tree (a list, a quote, emphasis); `syntax_tree` holds it with its kind and its span";

/// Text.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PROSE: &str = "text of the document: `prose` reads it through the run it is in; `syntax_tree` holds it as it is written";

/// Markup between events.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const MARKUP: &str = "syntax between the events of the parser, which read the construct it belongs to; `syntax_tree` holds it as it stands";

/// A section.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const SECTION: &str = "a heading and the blocks it heads, which hold no words of their own; `syntax_tree` and `syntax_child` hold it so a block's parent is its section, and `scope` reads the section by its heading";

/// White space.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space between the things the relations read; `syntax_tree` holds it as it stands";

/// Part of a construct read whole.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const THROUGH_PARENT: &str = "a part of a construct the relations read whole: `string_literal` reads the code block, `comment` the HTML block";

/// A construct of an extension the language leaves off.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const EXTENSION_OFF: &str = "a construct of an extension GitHub Flavored Markdown does not have, which the language leaves off: the parser never gives it";

/// Bytes that are not text.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "bytes that are not UTF-8 text: `syntax_error` reads what the language said of them";
