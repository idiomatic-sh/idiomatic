//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl. A kind of node or token is a node, a token or an extra: the Rust grammar's names as
//! rust-analyzer's parser spells them, in kebab case (`fn`, `path-segment`, `for-kw`,
//! `whitespace`); the element table holds its number.

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::RustNode;
use crate::domain::tokens::types::Token;

/// What a node or a token of a Rust syntax tree is: a node or a token of rust-analyzer's parser, or an extra it lacks. The element table
/// holds its number: the nodes first, then the tokens, then the extras, each in the order of its enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node.
    Node(RustNode),
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
    Node,
}

/// Whether an element is the root of its tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rootness {
    /// The root.
    Root,
    /// Anything below it.
    Inner,
}

/// Whether an element is inside an ancestor of a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Containment {
    /// An ancestor is of the kind.
    Inside,
    /// None is.
    Outside,
}

/// Whether an element has a child of a kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Offspring {
    /// It has one.
    Has,
    /// It has none.
    Lacks,
}

/// Whether an element is a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Typeness {
    /// A type.
    Type,
    /// Not one.
    NotType,
}

/// The text of an element as the file wrote it, borrowed from the file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct TokenText<'a>(pub &'a str);

/// Whether white space or a comment came between the last token of a name made of a file's tokens and the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Gap {
    /// No token is read yet.
    Start,
    /// None: the next token joins the last.
    Joined,
    /// Some: one space goes between them.
    Space,
}
