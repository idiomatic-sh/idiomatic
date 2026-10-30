//! The types of the `siblings` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What a token is to the roles of its siblings: trivia, a parenthesis, or a token that is written for what it says (an operator, a keyword).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Weight {
    /// White space, a comment, a line break or an indentation.
    Trivia,
    /// A parenthesis, which groups and says nothing of its own.
    Parenthesis,
    /// An operator, a keyword or another piece of punctuation.
    Written,
}

/// Whether a node stands after an element under the same parent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Placement {
    /// No node follows it.
    Last,
    /// A node follows it.
    Followed,
}

/// Whether a node has a token of a kind right under it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Holding {
    /// It has.
    Holds,
    /// It has not.
    Lacks,
}

/// How many nodes stand under a parent before an element: its place among the nodes, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SiblingIndex(pub usize);

/// A keyword or a piece of punctuation, as a token spells it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Word<'a>(pub &'a str);
