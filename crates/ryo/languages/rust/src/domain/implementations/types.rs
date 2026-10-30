//! The types of the `implementations` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

/// The header of an `impl`: its name, and where it is written, from its `impl` keyword to its body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    /// The header without its generics and where clause, single-spaced.
    pub name: languages::Symbol,
    /// Where it starts.
    pub from: languages::Offset,
    /// Where it ends.
    pub to: languages::Offset,
}

/// How far the reading of an `impl`'s header has come.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Opening {
    /// Before its `impl` keyword.
    Before,
    /// In the header, after the keyword.
    Header,
    /// At its body: the header is read.
    Body,
}
