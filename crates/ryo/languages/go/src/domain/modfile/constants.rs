//! The text the `modfile` concept reads a `go.mod` file with.

/// What opens a comment, to the end of its line.
pub(super) const COMMENT_OPEN: &[u8] = b"//";

/// What the language says of a block the file leaves open.
pub(super) const UNCLOSED: &str = "syntax error: the block is not closed: its `)` is missing";
