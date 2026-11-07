//! The words of the `definitions` concept: what a definition or a parameter is in Python's own words.

/// A function defined outside a class.
pub(crate) const FUNCTION: &str = "function";
/// A coroutine function defined outside a class.
pub(crate) const ASYNC_FUNCTION: &str = "async-function";
/// A function defined in a class body.
pub(crate) const METHOD: &str = "method";
/// A coroutine function defined in a class body.
pub(crate) const ASYNC_METHOD: &str = "async-method";
/// A class.
pub(crate) const CLASS: &str = "class";
/// A name `import` binds.
pub(crate) const IMPORT: &str = "import";
/// A name `from ... import` binds.
pub(crate) const FROM_IMPORT: &str = "from-import";
/// A name an assignment, a loop, a `with` or a comprehension binds.
pub(crate) const VARIABLE: &str = "variable";
/// The name a `type` statement binds.
pub(crate) const TYPE_ALIAS: &str = "type-alias";
/// The name an `except ... as` binds.
pub(crate) const EXCEPTION: &str = "exception";
/// A name a `case` pattern binds.
pub(crate) const PATTERN: &str = "pattern";
/// A type variable of a definition or an alias.
pub(crate) const TYPE_VAR: &str = "type-var";
/// A variadic type variable, `*Ts`.
pub(crate) const TYPE_VAR_TUPLE: &str = "type-var-tuple";
/// A parameter specification, `**P`.
pub(crate) const PARAM_SPEC: &str = "param-spec";
/// A parameter before `/`.
pub(crate) const POSITIONAL_ONLY: &str = "positional-only";
/// A parameter taken by position or by name.
pub(crate) const POSITIONAL_OR_KEYWORD: &str = "positional-or-keyword";
/// A parameter after `*` or `*args`.
pub(crate) const KEYWORD_ONLY: &str = "keyword-only";
/// `*args`.
pub(crate) const VAR_POSITIONAL: &str = "var-positional";
/// `**kwargs`.
pub(crate) const VAR_KEYWORD: &str = "var-keyword";
