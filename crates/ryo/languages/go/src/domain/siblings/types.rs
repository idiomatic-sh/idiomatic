//! The types of the `siblings` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::nodes::types::GoNode;
use crate::domain::tokens::types::Token;

/// Which siblings are counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wanted {
    /// Every node.
    GoNode,
    /// The nodes of one kind.
    NodeOf(GoNode),
    /// The tokens of one kind.
    Token(Token),
    /// The tokens that assign: `=`, `:=` and an operator with `=`.
    Assigning,
}

/// Whether an element is one a count wants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Match {
    /// It is.
    Wanted,
    /// It is not.
    Other,
}

/// Whether an element is trivia, which no field of `go/ast` holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Triviality {
    /// A comment, white space or bytes the parser could not read.
    Trivia,
    /// Any other element.
    Significant,
}

/// Whether an element stands at an end of its siblings, trivia left out: the first, or the last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standing {
    /// It does.
    Edge,
    /// It does not.
    Inside,
}
