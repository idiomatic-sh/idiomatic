//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What an element of a Python syntax tree is: a node of the standard, a token of the standard, or an extra no standard names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Kind {
    /// A node, by `CPython`'s `ast` name.
    Node(crate::domain::nodes::types::Node),
    /// A token, by the name of Python's `token` module.
    Token(crate::domain::tokens::types::Token),
    /// An element no standard names.
    Extra(crate::domain::extras::types::Extra),
}

/// Whether a string, an f-string or a t-string interpolates a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Interpolation {
    /// It does: its value is made when it runs.
    Interpolates,
    /// It does not: its value is the text written.
    Static,
}
