//! The types of the `visibilities` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use ryo::languages::{Offset, Position};

/// An export written on one declaration: its first and last keyword, and the declaration, by the node a `defines` row of it is known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Visible {
    /// The export that holds the keywords.
    pub holder: Position,
    /// Its first keyword, `export`.
    pub occurrence: Position,
    /// The declaration it is written on.
    pub declaration: Position,
    /// Where the keywords start.
    pub from: Offset,
    /// Where they end.
    pub to: Offset,
}

/// Whether a node declares a name of its own, which a `defines` row is known by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Declaring {
    /// It does.
    Declares,
    /// It does not.
    Silent,
}
