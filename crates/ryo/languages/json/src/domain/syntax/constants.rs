//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// A construct the relations read through its parts.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const THROUGH_PARTS: &str = "a construct the relations read through its parts: `entry` reads the name of a member and the value beside it";

/// White space.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TRIVIA: &str =
    "white space between the things the relations read; `syntax_tree` holds it as it stands";

/// A piece of punctuation.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str = "a piece of punctuation: syntax between the things the relations read, which read the construct it belongs to";

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the parser could not read as any token: `syntax_error` reads what the library said of it";

/// What opens a comment that runs to the end of the line.
pub(crate) const LINE_COMMENT_OPEN: &str = "//";

/// What opens a comment that closes with `*/`.
pub(crate) const BLOCK_COMMENT_OPEN: &str = "/*";

/// What closes a block comment.
pub(crate) const BLOCK_COMMENT_CLOSE: &str = "*/";

/// The base of every integer RFC 8259's grammar writes: it has no other.
pub(crate) const DECIMAL: i64 = 10;
