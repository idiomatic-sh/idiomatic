//! The names of the `syntax` concept: the standard macros whose arguments are expressions, and the reasons no relation models a kind of the
//! trees, which `sample` gives: those only the fixtures read are built with them.

/// The standard macros whose arguments are expressions: all but the markers are separated by commas, `vec!` also by a semicolon, and
/// `matches!` takes a pattern after the first. The syntax port reads the token tree of a call of one of them again as the expressions it
/// holds, and adds them under the call as an expansion.
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

/// A keyword.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const KEYWORD: &str =
    "a keyword: it introduces a construct, and the relations read the construct's own node";

/// An operator or a piece of punctuation.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PUNCTUATION: &str =
    "an operator or a piece of punctuation: syntax between the things the relations read";

/// A token of the lexer.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const LEXICAL: &str = "a token of the lexer (delimiter, number, string, trivia) that the relations read through the node that holds it";

/// A node that is an expression.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const EXPRESSION: &str = "an expression: the relations read the references, calls and literals inside it, never the expression itself";

/// A node that is a pattern.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PATTERN: &str =
    "a pattern: the relations read the names it binds as `defines`, never the pattern itself";

/// A node that is a type.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TYPE: &str = "a type: the relations read the paths in it as references and the annotation as written, never the type itself";

/// A node that groups others of one kind.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const LIST: &str =
    "a list that only groups its elements, which the relations read themselves";

/// A node of inline assembly.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const ASSEMBLY: &str = "inline assembly, which no relation models";

/// A node that is a part of a construct.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PART: &str =
    "a part of a construct, which the relations read through the construct's own node";

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
