//! The types of the `ruff_python_parser` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

/// How many nodes are open: zero outside the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Depth(pub usize);

/// A place among the bytes of the file, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ByteIndex(pub usize);

/// A place among the tokens of the library, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TokenIndex(pub usize);

/// The text the library reads for a file: the file's own bytes when they are UTF-8, or a copy with each byte that is not replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Readable<'a>(pub std::borrow::Cow<'a, str>);

/// The text the library decoded for a string's tokens: the value of a string literal, or the literal part of an f-string or a
/// t-string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Decoded<'a>(pub &'a str);

/// Which kind of string an interpolation is in, the innermost one open: what the standard names it by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Strings {
    /// An f-string, or none: a `FormattedValue`.
    Formatted,
    /// A t-string: an `Interpolation`.
    Template,
}

/// What `CPython`'s parser says where it refuses what the library reads without an error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Said(pub String);

/// A place `CPython`'s parser refuses and what it says there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ParserRefusal {
    /// Where.
    pub at: ruff_text_size::TextRange,
    /// What.
    pub said: Said,
}
