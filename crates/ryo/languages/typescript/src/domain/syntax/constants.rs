//! The scope of specialized readers beside the structural syntax relation.

/// Grammar kinds without a specialized semantic row remain visible through `syntax_tree`.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_GRAMMAR: &str = "This grammar child has no standalone semantic reader dispatch; readers reach it through its owner, and syntax_tree preserves its structure";

/// What opens a documentation comment, as `JSDoc` and TypeScript read it.
pub(crate) const DOC_OPEN: &str = "/**";
/// The empty block comment, which opens as a documentation comment does and documents nothing.
pub(crate) const EMPTY_BLOCK: &str = "/**/";
/// The name of a function's `this` parameter, which types the receiver and is no argument.
pub(crate) const RECEIVER: &str = "this";

/// The phase every `syntax_error` row names: the parser found the error.
pub(crate) const PARSE_PHASE: &str = "parse";

/// The impact every `syntax_error` row names: the recovered tree is incomplete.
pub(crate) const INCOMPLETE_IMPACT: &str = "incomplete";

/// The code every `syntax_error` row names: the parser gives no code.
pub(crate) const UNSPECIFIED_CODE: &str = "unspecified";

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";
