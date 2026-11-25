//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.





/// What opens a comment that runs to the end of the line.
pub(crate) const LINE_COMMENT_OPEN: &str = "//";

/// What opens a comment that closes with `*/`.
pub(crate) const BLOCK_COMMENT_OPEN: &str = "/*";

/// What closes a block comment.
pub(crate) const BLOCK_COMMENT_CLOSE: &str = "*/";

/// The base of every integer RFC 8259's grammar writes: it has no other.
pub(crate) const DECIMAL: i64 = 10;

/// The phase every `syntax_error` row names: the parser found the error.
pub(crate) const PARSE_PHASE: &str = "parse";

/// The impact every `syntax_error` row names: the recovered tree is incomplete.
pub(crate) const INCOMPLETE_IMPACT: &str = "incomplete";

/// The code every `syntax_error` row names: the parser gives no code.
pub(crate) const UNSPECIFIED_CODE: &str = "unspecified";

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";
