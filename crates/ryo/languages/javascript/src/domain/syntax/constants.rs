//! The names of the `syntax` concept: the reasons no relation models a kind of the trees, which `sample` gives.

/// Text the parser could not read.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const UNREAD_TEXT: &str =
    "text the parser could not read as any token: `syntax_error` reads what the library said of it";

/// The name a `CommonJS` import is called by.
pub(crate) const REQUIRE: &str = "require";
/// What opens a line comment.
pub(crate) const LINE_COMMENT: &str = "//";
/// What opens a hashbang line.
pub(crate) const HASHBANG: &str = "#!";
/// What opens a block comment.
pub(crate) const BLOCK_OPEN: &str = "/*";
/// What closes a block comment.
pub(crate) const BLOCK_CLOSE: &str = "*/";
/// What opens a documentation comment, as `JSDoc` and TypeScript read it.
pub(crate) const DOC_OPEN: &str = "/**";
/// The empty block comment, which opens as a documentation comment does and documents nothing.
pub(crate) const EMPTY_BLOCK: &str = "/**/";
/// What opens a parameter list.
pub(crate) const PARAMETERS_OPEN: &str = "(";
/// What closes a parameter list.
pub(crate) const PARAMETERS_CLOSE: &str = ")";
/// What opens a computed member name.
pub(crate) const COMPUTED_OPEN: &str = "[";
/// What separates the key of a property from its value, `a: 1`: a property without it is a method.
pub(crate) const PROPERTY_SEPARATOR: &str = ":";
/// What selects a member of a value.
pub(crate) const MEMBER_SEPARATOR: &str = ".";

/// What separates an arrow function's parameters from its body.
pub(crate) const ARROW: &str = "=>";
/// What starts the name of a private class member.
pub(crate) const PRIVATE_PREFIX: &str = "#";

/// The phase every `syntax_error` row names: the parser found the error.
pub(crate) const PARSE_PHASE: &str = "parse";

/// The impact every `syntax_error` row names: the recovered tree is incomplete.
pub(crate) const INCOMPLETE_IMPACT: &str = "incomplete";

/// The code every `syntax_error` row names: the parser gives no code.
pub(crate) const UNSPECIFIED_CODE: &str = "unspecified";

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";
