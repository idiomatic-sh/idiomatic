//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// What opens a comment that runs to the end of the line.
pub(crate) const LINE_COMMENT_OPEN: &str = "//";

/// What opens a comment that closes with `*/`.
pub(crate) const BLOCK_COMMENT_OPEN: &str = "/*";

/// What closes a block comment.
pub(crate) const BLOCK_COMMENT_CLOSE: &str = "*/";

/// The base of every integer RFC 8259's grammar writes: it has no other.
pub(crate) const DECIMAL: i64 = 10;

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";
