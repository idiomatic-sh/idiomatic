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

/// The node property that holds how an identifier reference uses its name, as the catalog's context word (`value`, `type`, `write`
/// or `read-write`), from the parser's semantic pass.
pub(crate) const REFERENCE_FLAGS: ryo::languages::PropertyId = ryo::languages::PropertyId(3);
/// The node property that holds, in decimal, where the name a resolved identifier reference resolves to is first declared: the start
/// of that declaration's binding identifier. An unresolved reference has none.
pub(crate) const REFERENCE_DECLARATION: ryo::languages::PropertyId = ryo::languages::PropertyId(4);
/// The node property that holds, in decimal, where a binding identifier that declares a name again first declared it: the start of
/// the first declaration's binding identifier. A name's first declaration has none.
pub(crate) const REDECLARATION_OF: ryo::languages::PropertyId = ryo::languages::PropertyId(5);
