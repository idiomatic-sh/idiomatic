//! The types of the `modfile` concept: what it reads a line of a `go.mod` file into.

use ryo::languages::Offset;

/// What a piece of a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PieceKind {
    /// Spaces, tabs and line ends.
    Space,
    /// A word: a verb, a module path or a version, quoted or not.
    Word,
    /// The `(` that opens a block.
    Open,
    /// The `)` that closes a block.
    Close,
    /// A `//` comment, to the end of its line.
    Comment,
}

/// A piece of a line: its kind and the bytes of the file it spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Piece {
    /// What it is.
    pub kind: PieceKind,
    /// Where it starts in the file.
    pub start: Offset,
    /// Where it ends in the file.
    pub end: Offset,
}

/// Whether a block is open after a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Opened {
    /// No block is open.
    Closed,
    /// A block is open, from where it starts: its lines follow, up to its `)`.
    Open(Offset),
}
