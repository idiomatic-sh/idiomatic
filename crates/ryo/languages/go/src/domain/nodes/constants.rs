//! The names of the `nodes` concept: the standard its nodes are named by, and why the relations read none of most of them.

/// The standard the nodes are named by: the syntax nodes `go/ast` defines, as the pinned version of Go lists them.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STANDARD: &str = "go/ast, Go 1.26.5";

/// A statement no relation reads by itself.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const STATEMENT: &str = "a statement no relation reads by itself: the relations read the declarations, calls, strings and names inside it; `syntax_tree` and `syntax_child` hold it as it stands";

/// An expression no relation reads by itself.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const EXPRESSION: &str = "an expression no relation reads by itself: the relations read the calls, strings and names inside it; `syntax_tree` and `syntax_child` hold it as it stands";

/// A type no relation reads by itself.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const TYPE: &str = "a type expression no relation reads by itself: `syntax_tree` and `syntax_child` hold it as it stands";

/// A part of a declaration the relations read through the declaration.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const PART: &str = "a part of a declaration the relations read through the declaration: a field list, a clause, a specification";

/// A name or a literal.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NAME: &str =
    "a name: the relations read the node it is in, and its text is the token under it";

/// A kind of the standard that no tree of the language holds.
#[cfg(any(test, feature = "fixtures"))]
pub(crate) const NEVER_WRITTEN: &str = "never in a tree the language writes: comments are tokens, text the parser could not read is an illegal token, and the files of a package are read one at a time";
