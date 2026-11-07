//! The names of the `tokens` concept: the standard its tokens are named by, and why the relations read none of most of them.

/// The standard the tokens are named by: the tokens `go/token` defines, as the pinned version of Go lists them. A token whose name is
/// a node's of `go/ast` has `-token` after it.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "go/token, Go 1.26.5";

/// An operator.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const OPERATOR: &str = "an operator: syntax between the things the relations read, which read the construct it belongs to";

/// A delimiter or a keyword.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str = "a keyword or a piece of punctuation: syntax between the things the relations read, which read the construct it belongs to";

/// The text of a name or of a number or character literal.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const LITERAL: &str =
    "the text of a name or of a number or character literal: the relations read the node it is in";

/// What a line comment starts with.
pub(crate) const LINE_COMMENT: &str = "//";

/// What a general comment starts with.
pub(crate) const BLOCK_COMMENT_OPEN: &str = "/*";

/// What a general comment ends with.
pub(crate) const BLOCK_COMMENT_CLOSE: &str = "*/";
