//! The types of the `visibilities` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo::languages::{Offset, Position};

/// A visibility written on one declaration: its first and last keyword, and the declaration, by the node its `defines` row is known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Visible {
    /// The node that holds the keywords: the export, or the member itself.
    pub(crate) holder: Position,
    /// The first keyword: `export`, or the accessibility.
    pub(crate) occurrence: Position,
    /// The declaration it is written on.
    pub(crate) declaration: Position,
    /// Where the keywords start.
    pub(crate) from: Offset,
    /// Where they end.
    pub(crate) to: Offset,
}

/// Whether a node declares a name of its own, which a `defines` row is known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declaring {
    /// It does.
    Declares,
    /// It does not.
    Silent,
}
