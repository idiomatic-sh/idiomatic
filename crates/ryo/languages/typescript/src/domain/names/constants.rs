//! TypeScript source delimiters and declaration keywords.
/// The name a `CommonJS` import is called by.
pub(crate) const REQUIRE: &str = "require";
/// What opens a line comment.
pub(crate) const LINE_COMMENT: &str = "//";
/// What opens a block comment.
pub(crate) const BLOCK_COMMENT: &str = "/*";
/// What closes a block comment.
pub(crate) const BLOCK_END: &str = "*/";
/// What opens a parameter list.
pub(crate) const OPEN_PARAMETERS: &str = "(";
/// What closes a parameter list.
pub(crate) const CLOSE_PARAMETERS: &str = ")";
/// What opens a computed member name.
pub(crate) const OPEN_COMPUTED: &str = "[";
/// What separates the key of a property from its value, `a: 1`: a property without it is a method.
pub(crate) const PROPERTY_SEPARATOR: &str = ":";
/// What selects a member of a value.
pub(crate) const SELECTION: &str = ".";
/// The keyword of a constant declaration.
pub(crate) const CONSTANT: &str = "const";
/// The keyword that names what a class or interface inherits.
pub(crate) const EXTENDS: &str = "extends";
