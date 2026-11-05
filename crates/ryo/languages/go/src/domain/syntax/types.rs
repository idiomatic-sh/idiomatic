//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor and no inherent impl. A kind of node or
//! token is a node, a token or an extra, named in kebab case (`func-decl`, `lparen`, `whitespace`); the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::GoNode;
use crate::domain::tokens::types::Token;

/// What a node or a token of a Go syntax tree is: a node of `go/ast`, a token of `go/token`, or an extra they lack. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(GoNode),
    /// A token.
    Token(Token),
    /// A kind the standard lacks.
    Extra(Extra),
}

/// Whether an element is a token, a leaf, or a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Leaf {
    /// A token.
    Token,
    /// A node.
    GoNode,
}

/// Whether an element is the root of its tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rootness {
    /// The root.
    Root,
    /// Anything below it.
    Inner,
}

/// Text of the file, as it is written: what an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text<'a>(pub &'a str);

/// A token of a node and its place among the elements of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Spelled {
    /// Where it is.
    pub at: ryo::languages::Position,
    /// Which token it is.
    pub token: Token,
}
