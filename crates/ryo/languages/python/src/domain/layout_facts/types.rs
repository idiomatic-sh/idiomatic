//! The types of the `layout_facts` concept: the line starts of a file.

use crate::domain::tokens::types::Token;
use ryo::languages::{Offset, Position};

/// Where each line of a file starts, in order: the first line starts at zero, every other after a line feed.
#[derive(Clone, Debug, Default)]
pub(crate) struct FileLines(pub Vec<Offset>);

/// A place in a file as an editor shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LinePlace {
    /// The line, from one.
    pub line: ryo::languages::Integer,
    /// The column: the bytes from the start of the line.
    pub column: ryo::languages::Integer,
}

/// A token of the file: its id and its kind.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TokenAt {
    /// Its id.
    pub id: Position,
    /// Its kind.
    pub token: Token,
}
