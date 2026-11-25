//! What the adapter's writer asks of the tree it writes into: the old element table's lossless tree, and the walk's node table.

use super::types::Piece;
use jsonc_parser::tokens::TokenAndRange;
use ryo::languages::Offset;

/// A tree the adapter writes the library's tokens and constructs into. Every byte before its end is in one token: the text no
/// token spans is written as white space, then as text the library could not read, before the next token.
pub(super) trait Sink {
    /// Opens a node of `piece` at `start`, after the gap before it.
    fn enter(&mut self, piece: Piece, start: Offset);
    /// Closes the innermost node at `end`.
    fn leave(&mut self, end: Offset);
    /// Writes a token the library scanned, a string with the text the library decoded from it.
    fn token(&mut self, scanned: &TokenAndRange<'_>);
    /// Writes the gap to `length`, the end of the text, and closes the file.
    fn close(&mut self, length: Offset);
    /// Records what is said of the file.
    fn diagnose(&mut self, message: &str, start: Offset, end: Offset);
}
