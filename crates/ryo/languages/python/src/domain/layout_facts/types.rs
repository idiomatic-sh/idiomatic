//! The types of the `layout_facts` concept: the line starts of a file, a token of it, and what each of its tokens is linked to.

use crate::domain::token_links::types::Pairing;
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
    pub(crate) id: Position,
    /// Its kind.
    pub(crate) token: Token,
}

/// What each token of a file is linked to, by the token's place among the ids after the file's nodes: the answer to "the links
/// of this token" with no scan of the tokens. Made in one pass over the tokens; a place holds the position of no node where a
/// token has no such link, and where the id is no token.
#[derive(Clone, Debug)]
pub(crate) struct LinkIndex {
    /// The id of the first place: the file's node count.
    pub(crate) first: Position,
    /// The token after each token.
    pub(crate) next: Vec<Position>,
    /// The token before each token.
    pub(crate) previous: Vec<Position>,
    /// The closing bracket of each opening bracket that has one, and the opening bracket of each such closing bracket.
    pub(crate) pair: Vec<Position>,
    /// The innermost open bracket around each token.
    pub(crate) around: Vec<Position>,
    /// Whether the pairing of the brackets held to the end of the file.
    pub(crate) pairing: Pairing,
}

impl Default for LinkIndex {
    fn default() -> Self {
        Self {
            first: Position(0),
            next: Vec::new(),
            previous: Vec::new(),
            pair: Vec::new(),
            around: Vec::new(),
            pairing: Pairing::Whole,
        }
    }
}
