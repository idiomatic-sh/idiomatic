//! The types of the `syntax` concept: plain data, public fields and derives, with no constructor and no inherent impl.

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

/// Text of the file, as it is written: what an element spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Text<'a>(pub &'a str);

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

/// What a `Constant` is, by what it is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    /// A string.
    String,
    /// A number.
    Number,
    /// Bytes, a boolean, `None`, `...`, or no constant.
    Other,
}

/// Whether an f-string stands as a string of its own or is the format spec of an interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standing {
    /// A string of its own.
    Own,
    /// The format spec of an interpolation: part of its string.
    FormatSpec,
}
