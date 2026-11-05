//! The types of the `pieces` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::grammar::types::{ByteIndex, Index};
use crate::domain::nodes::types::Node;
use crate::domain::tokens::types::Token;

/// Where a piece was found among the nodes, so that of two nodes over the same bytes the one found first holds the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Order(pub u32);

/// A node of `go/ast` found in the library's tree: its kind and the bytes it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Placed {
    /// What it is.
    pub kind: Node,
    /// Where it starts.
    pub start: ByteIndex,
    /// Where it ends.
    pub end: ByteIndex,
    /// Where it was found.
    pub order: Order,
}

/// The text a string literal stands for, kept as the pieces are found: every literal's text one after the other.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Decoded(pub String);

/// Where the text of one literal is in the decoded text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DecodedAt {
    /// Where it starts.
    pub from: ByteIndex,
    /// Where it ends.
    pub to: ByteIndex,
}

/// A token of `go/token` found in the library's tree: its kind, the bytes it spans, and where the text of its value is, for a string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Lexeme {
    /// What it is.
    pub kind: Token,
    /// Where it starts.
    pub start: ByteIndex,
    /// Where it ends.
    pub end: ByteIndex,
    /// Where the value of a string is.
    pub decoded: Option<DecodedAt>,
}

/// What the reading of one file finds: the nodes and the tokens, the text decoded for the strings, and the pointer types of the
/// library's tree that are the `*` of a conversion in `go/ast`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Pieces {
    /// The nodes of `go/ast`.
    pub nodes: Vec<Placed>,
    /// The tokens of `go/token`, in the order of the file.
    pub lexemes: Vec<Lexeme>,
    /// The values of the strings.
    pub decoded: Decoded,
    /// The pointer types read as the `*` of a conversion.
    pub consumed: Vec<Index>,
}
