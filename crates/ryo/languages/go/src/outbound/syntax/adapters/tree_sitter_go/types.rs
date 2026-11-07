//! The types of the `tree_sitter_go` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::grammar::types::ByteIndex;

/// A spelling the language has no token for: the text the library names an anonymous node by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unspelled<'a>(pub &'a str);

/// A kind of node the adapter has no form for: the library's name for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unrecognized<'a>(pub &'a str);

/// How far into the file the text the library reads starts: after a byte order mark, which `go/scanner` skips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Base(pub usize);

/// What the library says about a place of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Report {
    /// Where it starts.
    pub start: ByteIndex,
    /// Where it ends.
    pub end: ByteIndex,
    /// What is said.
    pub said: Said,
}

/// What a report says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Said {
    /// Text the parser could not read.
    Unreadable,
    /// A token the parser expected.
    Missing,
    /// A statement outside a function.
    OutsideFunction,
    /// A list that needs a comma before its line break.
    NeedsComma,
    /// A string with no closing quote.
    Unterminated,
}

/// What the library says of a node it reads in a place Go does not allow, which the library does not call an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Misreading {
    /// A statement outside a function.
    Statement,
    /// A list that ends with a line break after its last element and has no comma there.
    Comma,
    /// Nothing: the node is where Go has it.
    Fits,
}
