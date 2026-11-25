//! The names of the `syntax` concept: the built-in macros whose arguments are never code, and the reasons no relation models a kind of the
//! trees, which `sample` gives: those only the fixtures read are built with them.

/// The built-in macros whose arguments are never code: they read their tokens as text, a configuration predicate, a path or an
/// assembly template, so no expression in them is evaluated. The syntax port reads the token tree of a call of any other macro again
/// as the arguments it holds, and adds them under the call as an expansion.
pub(crate) const OPAQUE_MACROS: [&str; 20] = [
    "stringify",
    "concat",
    "concat_bytes",
    "concat_idents",
    "include",
    "include_str",
    "include_bytes",
    "env",
    "option_env",
    "cfg",
    "compile_error",
    "line",
    "column",
    "file",
    "module_path",
    "asm",
    "global_asm",
    "naked_asm",
    "trace_macros",
    "log_syntax",
];










/// Four slashes start an ordinary line comment.
pub(crate) const PLAIN_LINE_PREFIX: &str = "////";

/// An ordinary line comment opener.
pub(crate) const LINE_OPEN: &str = "//";

/// An outer line documentation opener.
pub(crate) const OUTER_LINE_DOC: &str = "///";

/// An inner line documentation opener.
pub(crate) const INNER_LINE_DOC: &str = "//!";

/// Three stars start an ordinary block comment.
pub(crate) const PLAIN_BLOCK_PREFIX: &str = "/***";

/// An empty ordinary block comment.
pub(crate) const EMPTY_BLOCK: &str = "/**/";

/// An ordinary block comment opener.
pub(crate) const BLOCK_OPEN: &str = "/*";

/// An outer block documentation opener.
pub(crate) const OUTER_BLOCK_DOC: &str = "/**";

/// An inner block documentation opener.
pub(crate) const INNER_BLOCK_DOC: &str = "/*!";

/// A block comment terminator.
pub(crate) const BLOCK_CLOSE: &str = "*/";

/// What goes between two tokens of a name made of a file's tokens that white space or a comment parted.
pub(crate) const SPACE: &str = " ";

/// The phase every `syntax_error` row names: the parser found the error.
pub(crate) const PARSE_PHASE: &str = "parse";

/// The impact every `syntax_error` row names: the recovered tree is incomplete.
pub(crate) const INCOMPLETE_IMPACT: &str = "incomplete";

/// The code every `syntax_error` row names: the parser gives no code.
pub(crate) const UNSPECIFIED_CODE: &str = "unspecified";

/// The configuration identity of a parse that takes no configuration.
pub(crate) const NO_CONFIGURATION: &str = "";

/// A tree the parser could not read completely has no structural class a query could compare.
pub(crate) const SHAPES_INCOMPLETE: &str = "expression shapes require a complete syntax tree";

/// A numeric literal a compiler refuses (a digit outside its base, past 128 bits, a suffix naming no numeric type) has no value.
pub(crate) const NUMBER_UNREAD: &str = "a numeric literal the compiler refuses has no value";

/// Macros with expression arguments in the released 0.1.4 grammar.
pub(crate) const EXPANDED_MACROS: [&str; 17] = [
    "format",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "vec",
    "matches",
    "panic",
    "todo",
];
